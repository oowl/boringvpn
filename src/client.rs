use bincode;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::io;
use dns_lookup;
use log::*;
use bincode::{serialize, deserialize};

use crate::device;
use crate::utils;
use crate::boring;
use crate::crypto::{Crypto,CryptoData,CryptoMethod};

type Token = u64;

pub struct Client {
    ip: IpAddr,
    netmask: IpAddr,
    token: Token,
    dns: IpAddr,
    tun: Option<device::Tuntap>
}


fn resolve(host: &str) -> Result<IpAddr, String> {
    let ip_list = dns_lookup::lookup_host(host).map_err(|_| "dns_lookup::lookup_host")?;
    Ok(ip_list.first().unwrap().clone())
}

impl Client {
    pub fn new() -> Self{
        Client {
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            netmask: IpAddr::V4(Ipv4Addr::new(255, 255, 255, 0)),
            token: 0,
            dns: IpAddr::V4(Ipv4Addr::new(114, 114, 114, 114)),
            tun: None
        }
    }

    fn parse_ip(&mut self,ipaddr: &str) -> Result<(),io::Error>{
        self.ip = ipaddr.parse().expect("failed to parse ipaddr from string");
        Ok(())
    }

    fn parse_netmask(&mut self,netmask: &str) -> Result<(),io::Error>{
        self.netmask = netmask.parse().expect("failed to parse netmask from string");
        Ok(())
    }


    fn set_token(&mut self,token: Token) {
        self.token = token
    }

    fn parse_dns(&mut self,dns: &str) -> Result<(),io::Error>{
        self.dns = dns.parse().expect("failed to parse ipaddr from string");
        Ok(())
    }

    pub fn create_tun(&mut self, ipaddr: &str,netmask: &str,dns: &str) -> Result<(),io::Error>{
        let tun = device::Tuntap::create("tun1", device::Type::Tun, None).expect("failed to create tun");
        self.parse_ip(ipaddr).unwrap();
        self.parse_dns(dns).unwrap();
        self.parse_netmask(netmask).unwrap();
        self.parse_dns(dns).expect("parse dns failed");
        tun.set_ip(&self.ip.to_string(),&self.netmask.to_string()).expect("failed to set ip to tun device");
        utils::set_dns(&self.dns.to_string()).expect("set dns failed");
        Ok(())
    }

    pub fn shakehand_udp(socket: &UdpSocket, addr: &SocketAddr, secret: &str) -> Result<(IpAddr,Token,String), String> {
        let request_msg = boring::Message::Request {msg: "hello".to_string() };
        let mut sender =  Crypto::from_shared_key(CryptoMethod::AES256, secret);
        let receiver = Crypto::from_shared_key(CryptoMethod::AES256, secret);
        let mut nonce = [0u8; 12];
        let encoded_req_msg: Vec<u8> = serialize(&request_msg).map_err(|e| e.to_string())?;
        let mut encrypted_req_msg = encoded_req_msg.clone();
        encrypted_req_msg.resize(encrypted_req_msg.len() + sender.additional_bytes(), 0);
        let add = [0u8; 8];
        let size = sender.encrypt(&mut encrypted_req_msg, encoded_req_msg.len(), &mut nonce, &add);

        while size > 0 {
        let sent_bytes = socket.send_to(&encrypted_req_msg, addr)
            .map_err(|e| e.to_string())?;
            size -= sent_bytes;
        }
        info!("Request sent to {}.", addr);
    }
}