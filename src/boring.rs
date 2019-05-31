use serde::{Serialize,Deserialize};
use std::net::IpAddr;


type Token = u64;

#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub enum Message {
    Request{msg: String},
    Response { ip: IpAddr,netmask: IpAddr,token: u64,dns: IpAddr},
    Data {ip: IpAddr,token: u64, data: Vec<u8>}
}


