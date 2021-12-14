# Alchemy Filtered Pending Transaction Notifications - Rust websocket client

Websocket client using tokio tungstenite (https://github.com/snapview/tokio-tungstenite) used for subscribing to pending transactions towards addresses on Ethereum mainnet. Supports subscribing to multiple addresses at the same time.

## Requirements

Requires Alchemy account which can be obtained here:
https://www.alchemy.com/

## Env variables used

WEB3_WS - Should be set to URL from Alchemy. For example wss://eth-mainnet.ws.alchemyapi.io/ws/12345678901234567890123456789012

RUST_LOG - Set up to use logging towards standard output. Set for example to info.

## Usage
cargo run --example opensea_example

Please see examples folder for full source code.

    
    // Create atomic bool which is used to tell connection if it should keep running. 
    // This is exposed so it can be shared between multiple processes
    let keep_alive = Arc::new(AtomicBool::new(true));

    // Create connection
    let connection = AlchemyPendingTransactionsWebsocket::connect(web3_ws_uri,keep_alive.clone()).await?;

    // Subscribe to address
    connection.subscribe_to_pending_tx_for("0x7be8076f4ea4a4ad08075c2508e481d6c946d12b").await?;

    // Poll for any new notifications
    let poll_result:Result<Option<Transaction>> = connection.poll_next();

    // Unsubscribe to address
    connection.unsubscribe_to_pending_tx_for("0x7be8076f4ea4a4ad08075c2508e481d6c946d12b").await?;

    // Disconnect from websocket
    connection.disconnect().await?;
## License
[MIT](https://choosealicense.com/licenses/mit/)