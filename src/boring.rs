use serde::{Serialize,Deserialize};


type Token = u64;

#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub enum Message {
    Request{msg: String},
    Response { ip: String,netmask: String,token: u64,dns: String},
    Data {ip: String,token: u64, data: Vec<u8>}
}


