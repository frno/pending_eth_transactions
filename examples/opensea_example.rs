#[macro_use]
extern crate log;

use anyhow::{Context, Result};

extern crate pending_eth_transactions;

use env_logger::{Builder, Target};
use pending_eth_transactions::AlchemyPendingTransactionsWebsocket;
use std::env;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    /*
    WEB3_WS needs to be set to url from Alchemy.
    RUST_LOG needs to be set (ex. info) so logging is shown.
    */

    // Set std out as log output
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

    // Keep alive is used as a kill switch
    let web3_ws_uri = env::var("WEB3_WS").context("Issue getting ENV Key [WEB3_WS]")?;
    let keep_alive = Arc::new(AtomicBool::new(true));
    let connection = AlchemyPendingTransactionsWebsocket::connect(web3_ws_uri,keep_alive.clone()).await?;

    // Subscribe to OpenSea
    connection.subscribe_to_pending_tx_for("0x7be8076f4ea4a4ad08075c2508e481d6c946d12b").await?;
    info!("Subscribed to OpenSea");

    info!("Waiting for OpenSea transactions for 10 seconds");
    // For two seconds print transaction hash for any new pending transaction
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        match connection.poll_next()? {
            None => {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Some(transaction) => {
                info!("New transaction - Hash: [{}]",transaction.hash);
            }
        }
    }

    info!("Unsubscribing and disconnecting");

    // Unsubscribe for messages
    connection.unsubscribe_to_pending_tx_for("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").await?;

    // Disconnect from websocket
    connection.disconnect().await?;

    return Ok(());
}