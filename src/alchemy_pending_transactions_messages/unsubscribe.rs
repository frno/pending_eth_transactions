use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use crate::alchemy_pending_transactions_messages::{increment_and_return, to_json};

pub fn create_unsubscribe_pending_tx_string(request_id:Arc<AtomicU64>, subscription_id:String) ->(u64, String){
    let new_request_id = increment_and_return(request_id);
    let result = UnSubscribe{
        id: new_request_id,
        method: "eth_unsubscribe".to_string(),
        jsonrpc: "2.0".to_string(),
        params: vec![subscription_id]
    };
    return (new_request_id, to_json(&result).unwrap());
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UnSubscribe {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Vec<String>,
}

#[cfg(test)]
mod unsubscribe_tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;
    use crate::alchemy_pending_transactions_messages::{create_unsubscribe_pending_tx_string, from_json, to_json};
    use crate::alchemy_pending_transactions_messages::UnSubscribe;

    #[test]
    fn deserialize_json() {
        let json = r#"{"jsonrpc":"2.0", "id": 1, "method": "eth_unsubscribe", "params": ["0x1a6ffa66f906bc88438bbf40593f4065"]}"#;
        let subscribe: UnSubscribe = from_json(json).unwrap();
        assert_eq!(subscribe.id, 1);
        assert_eq!(subscribe.jsonrpc, "2.0");
        assert_eq!(subscribe.method, "eth_unsubscribe");
        assert_eq!(subscribe.params.get(0).unwrap(), &String::from("0x1a6ffa66f906bc88438bbf40593f4065"));
    }

    #[test]
    fn serialize_json() {
        let result = UnSubscribe{
            id: 6,
            method: "eth_unsubscribe".to_string(),
            jsonrpc: "2.0".to_string(),
            params: vec![String::from("0x11111111111111111111111111111111")]
        };
        let to_json = to_json(&result).unwrap();
        assert_eq!(to_json.as_str(), r#"{"jsonrpc":"2.0","id":6,"method":"eth_unsubscribe","params":["0x11111111111111111111111111111111"]}"#);
    }

    #[test]
    fn create_unsubscribe_pending_tx(){
        let request_id:Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        let subscription_id = String::from("0x1a6ffa66f906bc88438bbf40593f4065");
        let (request_id,to_json) = create_unsubscribe_pending_tx_string(request_id, subscription_id);
        assert_eq!(to_json.as_str(), r#"{"jsonrpc":"2.0","id":1,"method":"eth_unsubscribe","params":["0x1a6ffa66f906bc88438bbf40593f4065"]}"#);
        assert_eq!(request_id, 1);
    }
}