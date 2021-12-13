# Alchemy Filtered Pending Transaction Notifications - Rust consumer

Websocket client using tokio tungstenite (https://github.com/snapview/tokio-tungstenite) used for subscribing to pending transactions towards addresses on Ethereum mainnet. Supports subscribing to multiple addresses at the same time.

## Requirements

Requires Alchemy account which can be obtained here:
https://www.alchemy.com/

## Env variables used

WEB3_WS - Should be set to URL from Alchemy. For example wss://eth-mainnet.ws.alchemyapi.io/ws/12345678901234567890123456789012

RUST_LOG - Set up to use logging towards standard output. Set for example to info.

## Usage
cargo run --example opensea_example

Please see examples folder for source code.

## License
[MIT](https://choosealicense.com/licenses/mit/)