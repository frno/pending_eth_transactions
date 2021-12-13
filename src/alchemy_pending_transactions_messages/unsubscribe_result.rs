use serde::{Serialize,Deserialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UnSubscribeResult {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(rename = "result")]
    pub success: bool,
}

#[cfg(test)]
mod unsubscribe_result_tests {
    use crate::alchemy_pending_transactions_messages::{from_json, to_json};
    use crate::alchemy_pending_transactions_messages::UnSubscribeResult;

    #[test]
    fn deserialize_json() {
        let json = r#"{"jsonrpc":"2.0","id":5,"result":false}"#;
        let subscription_result: UnSubscribeResult = from_json(json).unwrap();
        assert_eq!(subscription_result.id, 5);
        assert_eq!(subscription_result.jsonrpc, "2.0");
        assert_eq!(subscription_result.success, false);
    }

    #[test]
    fn serialize_json() {
        let result = UnSubscribeResult {
            id: 123,
            jsonrpc: "2.0".to_string(),
            success: true
        };
        let from_json = to_json(&result).unwrap();
        assert_eq!(r#"{"jsonrpc":"2.0","id":123,"result":true}"#,from_json.as_str());
    }
}