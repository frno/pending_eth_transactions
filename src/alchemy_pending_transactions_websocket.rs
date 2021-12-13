use anyhow::Result;
use flume::Sender;
use futures_util::Stream;
use futures_util::sink::SinkExt;
use parking_lot::Mutex;
use std::borrow::BorrowMut;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::task::Poll;
use thiserror::Error;
use tokio::macros::support::Pin;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use web3::types::{Address, Transaction};
use crate::alchemy_pending_transactions_messages::{AlchemyMessage, create_subscribe_pending_tx_string, create_unsubscribe_pending_tx_string, parse_to_alchemy_message};
use crate::block_util::convert_to_address;


/// Websocket connection towards Alchemy designed to subscribe to pending transactions for
/// specified addresses.
/// https://docs.alchemy.com/alchemy/guides/using-websockets#2-alchemy_filterednewfullpendingtransactions
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicBool;
/// use tokio::time::{Duration, Instant};
///
/// // Keep alive is used as a kill switch
/// let keep_alive = Arc::new(AtomicBool::new(true));
/// let url = "wss://eth-mainnet.ws.alchemyapi.io/ws/12345678901234567890123456789012";
/// let alchemy = AlchemyPendingTransactionsWebsocket::connect(url,keep_alive.clone()).await?;
///
/// // Subscribe to USDC contract address
/// alchemy.subscribe_to_pending_tx_for("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").await;
///
/// // For ten seconds print transaction hash for any new pending transaction
/// let start = Instant::now();
/// while start.elapsed() < Duration::from_secs(10) {
///     match alchemy.poll_next() {
///         None => {
///                 tokio::time::sleep(Duration::from_millis(25)).await;
///         }
///         Some(transaction) => {
///             println!("Hash: [{}]",transaction.hash);
///         }
///     }
/// }
///
/// // Unsubscribe for messages
/// alchemy.unsubscribe_to_pending_tx_for("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").await;
///
/// // Disconnect from websocket
/// alchemy.disconnect().await;
/// ```
pub struct AlchemyPendingTransactionsWebsocket{
    subscriptions_active: Arc<Mutex<HashMap<Address,String>>>,
    read_handle: Arc<Mutex<Option<JoinHandle<Result<()>>>>>,
    last_request_id: Arc<AtomicU64>,
    keep_alive: Arc<AtomicBool>,
    requests_pending_result: Arc<Mutex<HashMap<u64,Sender<Result<AlchemyMessage>>>>>,
    messages_to_send: Arc<Mutex<VecDeque<Message>>>,
    pending_transactions: Arc<Mutex<VecDeque<Transaction>>>
}

impl AlchemyPendingTransactionsWebsocket{
    /// Takes a request to connect and returns a new instance if we are able to connect.
    /// First connects to Alchemy using wss
    /// then starts thread to read messages
    /// only returns if able to connect
    pub async fn connect<Req: IntoClientRequest>(
        request: Req,
        keep_alive: Arc<AtomicBool>,
    )-> Result<AlchemyPendingTransactionsWebsocket>{
        // Create a shared request id, so each request gets a unique request id
        let last_request_id:Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

        // Create a hashmap between requests id pending response and their sender channel
        let requests_pending_result:HashMap<u64,Sender<Result<AlchemyMessage>>> = HashMap::new();
        let requests_pending_result = Arc::new(Mutex::new(requests_pending_result));

        // Create queue for messages to send
        let messages_to_send = VecDeque::new();
        let messages_to_send = Arc::new(Mutex::new(messages_to_send));

        // Create queue for transactions
        let pending_transactions = VecDeque::new();
        let pending_transactions = Arc::new(Mutex::new(pending_transactions));

        // Connect up towards Alchemy
        let new_request = request.into_client_request()?;
        let (ws, _) = connect_async(new_request).await?;

        // Clone shared object being sent to task
        let io_task_requests_pending_result = requests_pending_result.clone();
        let io_task_messages_to_send = messages_to_send.clone();
        let io_task_pending_transactions = pending_transactions.clone();
        let io_task_keep_alive = keep_alive.clone();

        // Spawn separate task for reading messages
        let read_handle = tokio::spawn(async move {
            let result = AlchemyPendingTransactionsWebsocket::io_loop(ws,
                                                                      io_task_messages_to_send,
                                                                      io_task_pending_transactions,
                                                                      io_task_requests_pending_result,
                                                                      io_task_keep_alive.clone()).await;
            io_task_keep_alive.swap(false, Ordering::Relaxed);
            return match result {
                Ok(_) => { Ok(()) }
                Err(err) => { Err(err) }
            }
        });
        let read_handle = Arc::new(Mutex::new(Some(read_handle)));

        let subscriptions_active:HashMap<Address,String> = HashMap::new();
        let subscriptions_active = Arc::new(Mutex::new(subscriptions_active));

        return Ok(AlchemyPendingTransactionsWebsocket{
            read_handle,
            last_request_id,
            pending_transactions,
            messages_to_send,
            requests_pending_result,
            subscriptions_active,
            keep_alive
        });
    }

    /// Disconnects the web socket connection
    /// Does not return until read task has exited
    /// If read thread does not shutdown cleanly will return ReadError
    pub async fn disconnect(&self)->Result<()>{
        self.keep_alive.swap(false, Ordering::Relaxed);
        let handle = self.read_handle.lock().take();
        return match handle {
            None => {
                Ok(())
            }
            Some(handle) => {
                return handle.await?;
            }
        }
    }

    /// Take address or string for address and then send subscription to websocket.
    /// returns Ok if able to subscribe
    pub async fn subscribe_to_pending_tx_for<AdrReq: IntoWeb3Address>(&self, address: AdrReq) ->Result<()>{
        self.ensure_is_connected()?;
        let address = address.into_address()?;
        let (request_id, subscribe) = create_subscribe_pending_tx_string(self.last_request_id.clone(), address);
        let subscribe_result = self.send_request_get_response(request_id,subscribe).await?;
        match subscribe_result{
            AlchemyMessage::SubscribeResult(result) => {
                if result.id == request_id {
                    // Add mapping between address and subscription id
                    let mut subs = self.subscriptions_active.lock();
                    subs.insert(address,result.subscription_id);
                }else{
                    return Err(anyhow::Error::from(AlchemyWebsocketError::ReceivedResponseForUnexpectedMessageIdOnSubscribe))
                }
            }
            _ => {
                return Err(anyhow::Error::from(AlchemyWebsocketError::ReceivedUnexpectedOnSubscribe))
            }
        }

        Ok(())
    }

    /// Take address or string for address and then if subscribed sends unsubscribe to websocket.
    /// returns Ok if able to unsubscribe
    pub async fn unsubscribe_to_pending_tx_for<AdrReq: IntoWeb3Address>(&self, address: AdrReq) ->Result<()>{
        self.ensure_is_connected()?;
        let subbed_address = address.into_address()?;
        let subscribe_result = self.get_subscribe_id(subbed_address)?;
        if subscribe_result.is_none() {
            // Allready unsubscribed
            return Ok(());
        }
        let subscribe_id = subscribe_result.unwrap();
        let (request_id, subscribe) = create_unsubscribe_pending_tx_string(self.last_request_id.clone(), subscribe_id);
        let subscribe_result = self.send_request_get_response(request_id,subscribe).await?;
        match subscribe_result{
            AlchemyMessage::UnSubscribeResult(result) => {
                if result.id == request_id {
                    // Remove mapping between address and subscription id
                    let mut subs = self.subscriptions_active.lock();
                    subs.remove(&subbed_address);
                }else{
                    return Err(anyhow::Error::from(AlchemyWebsocketError::ReceivedResponseForUnexpectedMessageIdOnUnSubscribe))
                }
            }
            _ => {
                return Err(anyhow::Error::from(AlchemyWebsocketError::ReceivedUnexpectedOnUnSubscribe))
            }
        }
        Ok(())
    }

    /// Take address or string for address and then checks if we are subscribed for pending tx
    /// returns true if subscribed and false if not
    pub fn get_subscribe_id<AdrReq: IntoWeb3Address>(&self, address: AdrReq) ->Result<Option<String>>{
        self.ensure_is_connected()?;
        let address = address.into_address()?;
        let subs = self.subscriptions_active.lock();
        match subs.contains_key(&address){
            true => {
                let sub_id = subs.get(&address).unwrap();
                Ok(Some(String::from(sub_id)))
            }
            false => { Ok(None) }
        }
    }

    /// Checks if Transaction is available in queue if not then returns None
    /// else consumes Transaction from queue
    pub fn poll_next(&self)->Result<Option<Transaction>>{
        self.ensure_is_connected()?;
        let mut pending_transactions = self.pending_transactions.lock();
        return Ok(pending_transactions.pop_front());
    }

    /// Check if stream is still connected. Actually checks if io loop is still running which should be same
    pub fn is_connected(&self)->bool{
        return self.keep_alive.load(Ordering::Relaxed);
    }

    /// before actions are performed check we are connected
    fn ensure_is_connected(&self)->Result<()>{
        match self.keep_alive.load(Ordering::Relaxed){
            true => { return Ok(()); }
            false => { return Err(anyhow::Error::from(AlchemyWebsocketError::ConnectionClosed))}
        }
    }


    /// General IO loop for the websocket stream
    /// Checks to see if should be running
    /// Polls for messages towards websocket stream
    ///     Handles message (send response to requests, ping to pong)
    /// Checks for outgoing messages
    ///     Sends message to stream
    /// If handling nothing, perform a sleep
    async fn io_loop(mut ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
                     messages_to_send: Arc<Mutex<VecDeque<Message>>>,
                     pending_transactions: Arc<Mutex<VecDeque<Transaction>>>,
                     requests_pending_result:Arc<Mutex<HashMap<u64,Sender<Result<AlchemyMessage>>>>>,
                     keep_alive: Arc<AtomicBool>) ->Result<()>{
        let mut data = Pin::new(&mut ws);
        loop{
            if !keep_alive.load(Ordering::Relaxed) {
                warn!("Breaking task due to signaling");
                return Ok(());
            }

            let mut performed_read_or_write = false;

            let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());
            let poll = data.as_mut().poll_next(cx.borrow_mut());
            let message_result = AlchemyPendingTransactionsWebsocket::fetch_message_from_poll(poll).await?;
            match message_result {
                None => {}
                Some(message) => {
                    AlchemyPendingTransactionsWebsocket::handle_incoming_message(message,
                                                                                 &messages_to_send,
                                                                                 &pending_transactions,
                                                                                 &requests_pending_result)?;
                    performed_read_or_write = true;
                }
            }

            //dequeue message and send to websocket
            let message = AlchemyPendingTransactionsWebsocket::dequeue_message(&messages_to_send);
            if let Some(msg) = message{
                performed_read_or_write = true;
                data.send(msg).await?;
            }

            if performed_read_or_write == false{
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }

    // If there are any message then return them, else dequeue
    fn dequeue_message(messages_to_send: &Arc<Mutex<VecDeque<Message>>>)->Option<Message>{
        let mut queue = messages_to_send.lock();
        return queue.pop_front();
    }

    fn handle_incoming_message(message:Message,
                               messages_to_send: &Arc<Mutex<VecDeque<Message>>>,
                               pending_transactions: &Arc<Mutex<VecDeque<Transaction>>>,
                               requests_pending_result: &Arc<Mutex<HashMap<u64,Sender<Result<AlchemyMessage>>>>>)->Result<()>{
        match message{
            Message::Text(string) => {
                let alchemy_message = parse_to_alchemy_message(string);
                let mut id:Option<u64> = None;
                match alchemy_message{
                    AlchemyMessage::SubscribeResult(ref res) =>{
                        let requests = requests_pending_result.lock();
                        match requests.contains_key(&res.id){
                            true => {
                                id = Some(res.id);
                            }
                            false => {
                                return Err(anyhow::Error::from(AlchemyWebsocketError::ReceivedSubscribeResultForUnexpectedMessageId))
                            }
                        }
                    }
                    AlchemyMessage::UnSubscribeResult(ref res) => {
                        let requests = requests_pending_result.lock();
                        match requests.contains_key(&res.id){
                            true => {
                                id = Some(res.id);
                            }
                            false => {
                                return Err(anyhow::Error::from(AlchemyWebsocketError::ReceivedUnSubscribeResultForUnexpectedMessageId))
                            }
                        }
                    }
                    AlchemyMessage::PendingTransactionSubscription(ref pending_transaction_subscription_message) => {
                        let pending_transaction = (&pending_transaction_subscription_message.params.transaction).clone();
                        let mut pending_transactions = pending_transactions.lock();
                        pending_transactions.push_back(pending_transaction);
                    }
                    AlchemyMessage::Unknown(ref msg) => {
                        warn!("Received text that could not parsed to alchemy message [{}]",msg);
                    }
                }
                if id.is_some() {
                    let mut requests = requests_pending_result.lock();
                    let request_id = id.unwrap();
                    let receive = requests.get(&request_id).unwrap();
                    receive.send(Ok(alchemy_message))?;
                    requests.remove(&request_id);
                }

            }
            Message::Ping(v) => {
                AlchemyPendingTransactionsWebsocket::add_message_to_send(Message::Pong(v), &messages_to_send);
            }
            Message::Pong(_) => {}
            Message::Binary(_) => {
                // Not expected!
                warn!("Received binary data on websocket. Unexpected.")
            }
            Message::Close(_) => {
                return Err(anyhow::Error::from(AlchemyWebsocketError::ConnectionClosed))
            }
        }
        return Ok(());
    }

    /// Add message to send queue
    fn add_message_to_send(message:Message, messages_to_send: &Arc<Mutex<VecDeque<Message>>>){
        let mut queue = messages_to_send.lock();
        queue.push_back(message);
    }

    /// Takes a poll and returns a Message if ready, if not returns None, will yield error if poll's result is erroneous
    async fn fetch_message_from_poll(try_next: Poll<Option<Result<Message, tokio_tungstenite::tungstenite::Error>>>)->Result<Option<Message>>{
        return match try_next {
            Poll::Ready(result) => {
                match result {
                    None => {
                        Ok(None)
                    }
                    Some(try_next) => {
                        match try_next {
                            Ok(msg) => {
                                Ok(Some(msg))
                            }
                            Err(err) => {
                                Err(anyhow::Error::from(err))
                            }
                        }
                    }
                }
            }
            Poll::Pending => {
                Ok(None)
            }
        }
    }


    /// Add the pending request id to the struct's map of request id to sender of response
    fn add_requests_pending_result(&self, request_id:u64, tx_response: Sender<Result<AlchemyMessage>>){
        let mut mapping = self.requests_pending_result.lock();
        mapping.insert(request_id, tx_response);
    }

    /// Send a string to websocket and wait for a response
    async fn send_request_get_response(&self, request_id:u64, request:String)->Result<AlchemyMessage>{
        // Add request to map of pending requests
        let (tx_response, rx_response) = flume::unbounded();
        self.add_requests_pending_result(request_id, tx_response);
        // Add to send queue
        AlchemyPendingTransactionsWebsocket::add_message_to_send(Message::Text(request), &self.messages_to_send);
        // Wait for response
        let inner_response = rx_response.recv_async().await;
        if inner_response.is_err() {
            let err = inner_response.err().unwrap();
            error!("Error on reading from inner response async! [{}]", err);
            return Err(anyhow::Error::from(err));
        }
        return inner_response.unwrap();
    }
}

pub trait IntoWeb3Address {
    /// Convert into a web3 Address.
    fn into_address(self) -> Result<Address>;
}

impl IntoWeb3Address for &str {
    fn into_address(self) -> Result<Address> {
        return convert_to_address(self);
    }
}

impl IntoWeb3Address for String {
    fn into_address(self) -> Result<Address> {
        return convert_to_address(self.as_str());
    }
}

impl IntoWeb3Address for Address {
    fn into_address(self) -> Result<Address> {
        Ok(self)
    }
}


#[derive(Error, Debug)]
pub enum AlchemyWebsocketError {
    #[error("connection closed")]
    ConnectionClosed,
    #[error("received unexpected message as response to subscribe")]
    ReceivedUnexpectedOnSubscribe,
    #[error("received unexpected message as response to unsubscribe")]
    ReceivedUnexpectedOnUnSubscribe,
    #[error("received unexpected message id in response to subscribe")]
    ReceivedResponseForUnexpectedMessageIdOnSubscribe,
    #[error("received unexpected message id in response to unsubscribe")]
    ReceivedResponseForUnexpectedMessageIdOnUnSubscribe,
    #[error("received subscribe result for unexpected message id")]
    ReceivedSubscribeResultForUnexpectedMessageId,
    #[error("received unsubscribe result for unexpected message id")]
    ReceivedUnSubscribeResultForUnexpectedMessageId,
}
