use serde::{Serialize,Deserialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SubscribeResult {
    pub id: u64,
    #[serde(rename = "result")]
    pub subscription_id: String,
    pub jsonrpc: String,
}

#[cfg(test)]
mod subscription_result_tests {
    use crate::alchemy_pending_transactions_messages::{from_json, to_json};
    use crate::alchemy_pending_transactions_messages::SubscribeResult;

    #[test]
    fn deserialize_json() {
        let json = r#"{"id":1,"result":"0xc3217453acdd4a6339f93822f9237e3b","jsonrpc":"2.0"}"#;
        let subscription_result: SubscribeResult = from_json(json).unwrap();
        assert_eq!(subscription_result.id, 1);
        assert_eq!(subscription_result.jsonrpc, "2.0");
        assert_eq!(subscription_result.subscription_id, "0xc3217453acdd4a6339f93822f9237e3b");
    }

    #[test]
    fn serialize_json() {
        let result = SubscribeResult {
            id: 123,
            subscription_id: "0xc1111111222222223333334444445555".to_string(),
            jsonrpc: "2.0".to_string()
        };
        let from_json = to_json(&result).unwrap();
        assert_eq!(r#"{"id":123,"result":"0xc1111111222222223333334444445555","jsonrpc":"2.0"}"#,from_json.as_str());
    }
}