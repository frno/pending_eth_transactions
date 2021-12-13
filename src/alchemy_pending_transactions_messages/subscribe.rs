use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64};
use web3::types::Address;
use crate::alchemy_pending_transactions_messages::{increment_and_return, to_json};

pub fn create_subscribe_pending_tx_string(request_id:Arc<AtomicU64>, address:Address)->(u64, String){
    let new_request_id = increment_and_return(request_id);
    let address = format!("{:?}", address);
    let result = Subscribe{
        id: new_request_id,
        method: "eth_subscribe".to_string(),
        jsonrpc: "2.0".to_string(),
        params: ("alchemy_filteredNewFullPendingTransactions".to_string(), SubscribeParams { address })
    };
    return (new_request_id, to_json(&result).unwrap());
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Subscribe {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: (String, SubscribeParams),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SubscribeParams {
    pub address: String,
}

#[cfg(test)]
mod subscribe_tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;
    use web3::types::Address;
    use crate::alchemy_pending_transactions_messages::{create_subscribe_pending_tx_string, from_json, SubscribeParams, to_json};
    use crate::alchemy_pending_transactions_messages::Subscribe;
    use crate::convert_to_address;

    #[test]
    fn deserialize_json() {
        let json = r#"{"jsonrpc":"2.0","id": 1, "method": "eth_subscribe", "params": ["alchemy_filteredNewFullPendingTransactions", {"address": "0x7be8076f4ea4a4ad08075c2508e481d6c946d12b"}]}"#;
        let subscribe: Subscribe = from_json(json).unwrap();
        assert_eq!(subscribe.id, 1);
        assert_eq!(subscribe.jsonrpc, "2.0");
        assert_eq!(subscribe.method, "eth_subscribe");
        assert_eq!(subscribe.params.0, "alchemy_filteredNewFullPendingTransactions");
        assert_eq!(subscribe.params.1.address, "0x7be8076f4ea4a4ad08075c2508e481d6c946d12b");
    }

    #[test]
    fn serialize_json() {
        let result = Subscribe{
            id: 1,
            method: "eth_subscribe".to_string(),
            jsonrpc: "2.0".to_string(),
            params: ("alchemy_filteredNewFullPendingTransactions".to_string(), SubscribeParams { address: "0x7be8076f4ea4a4ad08075c2508e481d6c946d12b".to_string() })
        };
        let to_json = to_json(&result).unwrap();
        assert_eq!(to_json.as_str(), r#"{"jsonrpc":"2.0","id":1,"method":"eth_subscribe","params":["alchemy_filteredNewFullPendingTransactions",{"address":"0x7be8076f4ea4a4ad08075c2508e481d6c946d12b"}]}"#);
    }

    #[test]
    fn create_subscribe_pending_tx(){
        let request_id:Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        let address:Address = convert_to_address("0x7be8076f4ea4a4ad08075c2508e481d6c946d12b").unwrap();
        let (request_id,to_json) = create_subscribe_pending_tx_string(request_id, address);
        assert_eq!(to_json.as_str(), r#"{"jsonrpc":"2.0","id":1,"method":"eth_subscribe","params":["alchemy_filteredNewFullPendingTransactions",{"address":"0x7be8076f4ea4a4ad08075c2508e481d6c946d12b"}]}"#);
        assert_eq!(request_id, 1);
    }
}