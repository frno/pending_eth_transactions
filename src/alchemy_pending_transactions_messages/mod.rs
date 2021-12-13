use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use anyhow::Result;

mod subscribe;
pub use subscribe::*;

mod subscribe_result;
pub use subscribe_result::*;

mod pending_transaction;
pub use pending_transaction::*;

mod unsubscribe;
pub use unsubscribe::*;

mod unsubscribe_result;
pub use unsubscribe_result::*;

pub fn to_json<T>(arg: &T)->Result<String> where
    T: ?Sized + serde::ser::Serialize, {
    return match serde_json::to_string(arg){
        Ok(v)=>{ Ok(v)},
        Err(e)=>{ Err(anyhow::Error::from(e))}
    }
}

pub fn from_json<'a, T>(json: &'a str) ->Result<T> where
    T: serde::de::Deserialize<'a>, {
    return match serde_json::from_str(json){
        Ok(head) => {Ok(head)}
        Err(err) => { Err(anyhow::Error::from(err))}
    }
}

fn increment_and_return(request_id:Arc<AtomicU64>)->u64{
    let last_request_id = request_id.load(Ordering::Relaxed);
    let new_request_id = last_request_id +1;
    request_id.swap(new_request_id, Ordering::Relaxed);
    return new_request_id;
}

pub enum AlchemyMessage{
    SubscribeResult(SubscribeResult),
    UnSubscribeResult(UnSubscribeResult),
    PendingTransactionSubscription(PendingTransactionSubscription),
    Unknown(String),
}

pub fn parse_to_alchemy_message(json: String)->AlchemyMessage{
    let res: Result<PendingTransactionSubscription> = from_json(json.as_str());
    if res.is_ok(){
        return AlchemyMessage::PendingTransactionSubscription(res.unwrap());
    }

    let res: Result<UnSubscribeResult> = from_json(json.as_str());
    if res.is_ok(){
        return AlchemyMessage::UnSubscribeResult(res.unwrap());
    }

    let res: Result<SubscribeResult> = from_json(json.as_str());
    if res.is_ok(){
        return AlchemyMessage::SubscribeResult(res.unwrap());
    }

    return AlchemyMessage::Unknown(json);
}