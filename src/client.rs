use bincode;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::io::{self,Write,Read};
use dns_lookup;
use log::*;
use bincode::{serialize, deserialize};
use std::os::unix::io::AsRawFd;
use mio;


use crate::device;
use crate::utils;
use crate::boring;
use crate::crypto::{Crypto,CryptoData,CryptoMethod};
use crate::types::Error;

type Token = u64;

#[derive(Debug,Clone)]
pub struct Client {
    ip: IpAddr,
    netmask: IpAddr,
    token: Token,
    dns: IpAddr,
    secret: String,
    host: IpAddr,
    port: u16,
    default_route: bool
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
            secret: String::new(),
            host: IpAddr::V4(Ipv4Addr::new(114, 114, 114, 114)),
            port: 0 as u16,
            default_route: false
        }
    }

    fn parse_ip(&mut self,ipaddr: &str) -> Result<(),Error>{
        self.ip = ipaddr.parse().map_err(|e| Error::Parse("failed to parse ipaddr from string",e))?;
        Ok(())
    }

    fn parse_netmask(&mut self,netmask: &str) -> Result<(),Error>{
        self.netmask = netmask.parse().map_err(|e| Error::Parse("failed to parse netmask from string",e))?;
        Ok(())
    }

    pub fn parse_host(&mut self,host: &str) -> Result<(),Error>{
        self.netmask = host.parse().map_err(|e| Error::Parse("failed to parse host from string",e))?;
        Ok(())
    }

    pub fn parse_key(&mut self,key: &str) {
        self.secret = key.to_string()
    }

    pub fn parse_default_route(&mut self,default: bool) {
        self.default_route = default;
    }

    pub fn parse_port(&mut self,port: u16) {
        self.port = port;
    }

    fn set_token(&mut self,token: Token) {
        self.token = token
    }

    fn parse_dns(&mut self,dns: &str) -> Result<(),Error>{
        self.dns = dns.parse().map_err(|e| Error::Parse("failed to parse dns from string",e))?;
        Ok(())
    }

    pub fn create_tun(&mut self) -> Result<device::Tuntap,io::Error>{
        let tun = device::Tuntap::create("tun1", device::Type::Tun, None).expect("failed to create tun");
        // self.parse_ip(ipaddr).unwrap();
        // self.parse_dns(dns).unwrap();
        // self.parse_netmask(netmask).unwrap();
        // self.parse_dns(dns).expect("parse dns failed");
        tun.set_ip(&self.ip.to_string(),&self.netmask.to_string()).expect("failed to set ip to tun device");
        utils::set_dns(&self.dns.to_string()).expect("set dns failed");
        Ok(tun)
    }

    pub fn shakehand_udp(&mut self,socket: &UdpSocket, addr: &SocketAddr) -> Result<(Crypto,Crypto), Error> {
        let request_msg = boring::Message::Request {msg: "hello".to_string() };
        let mut sender =  Crypto::from_shared_key(CryptoMethod::AES256, &self.secret);
        let receiver = Crypto::from_shared_key(CryptoMethod::AES256, &self.secret);
        let mut nonce = [0u8; 12];
        let encoded_req_msg: Vec<u8> = serialize(&request_msg).unwrap();
        let mut encrypted_req_msg = encoded_req_msg.clone();
        encrypted_req_msg.resize(encrypted_req_msg.len() + sender.additional_bytes(), 0);
        let add = [0u8; 8];
        let mut size = sender.encrypt(&mut encrypted_req_msg, encoded_req_msg.len(), &mut nonce, &add);

        while size > 0 {
            let sent_bytes = socket.send_to(&encrypted_req_msg, addr).map_err(|e| Error::Shakehand("failed send handshake",e))?;
            size -= sent_bytes;
        }
        info!("Request sent to {}.", addr);
        let mut buf = [0u8; 1600];
        let (len, recv_addr) = socket.recv_from(&mut buf).map_err(|e| Error::Shakehand("failed recv_from shakehand",e))?;
        assert_eq!(&recv_addr, addr);
        info!("Response received from {}.", addr);

        let decrypted_buf_len = receiver.decrypt(&mut buf[..len], &nonce, &add).map_err(|e| e)?;
        let resp_msg: boring::Message = deserialize(&buf[..decrypted_buf_len]).unwrap();
        match resp_msg {
            boring::Message::Response { ip, netmask,token, dns } => {
                self.ip = ip;
                self.netmask = netmask;
                self.set_token(token);
                self.dns = dns;
                Ok((sender,receiver))
            },
            _ => Err(
                Error::Invaildmessage("error shakehand message")
            ),
        }   
    }

    pub fn shakehand_tcp() {
        unimplemented!()
    }

    pub fn connect_udp(&mut self,default_route: bool) -> Result<(),Error> {
        info!("start connect server");
        let remote_ip = self.host;
        let remote_addr = SocketAddr::new(remote_ip, self.port);
        info!("remote addr and port is {}:{}",remote_ip,self.port);

        let local_addr: SocketAddr = "0.0.0.0:0".parse::<SocketAddr>().unwrap();
        let socket = UdpSocket::bind(&local_addr).unwrap();
        let (mut sender,receiver) = self.shakehand_udp(&socket, &remote_addr).unwrap();
        info!("shakehand sucess token: {}, ip address: {}",self.token,self.ip.to_string());
        info!("start create tun device");
        let mut tun = self.create_tun().unwrap();
        info!("tun device create successful,set ip: {} netmask: {}",self.ip.to_string(),self.netmask.to_string());
        tun.up().unwrap();
        let tun_rawfd = tun.as_raw_fd();

        let mut buf = [0u8; 1600];
        let mut nonce = [0u8; 12];
        let add = [0u8; 8];

        let tunfd = mio::unix::EventedFd(&tun_rawfd);
        let sockfd = mio::net::UdpSocket::from_socket(socket).unwrap();

        info!("start polling...");
        const TUN_TOKEN: mio::Token = mio::Token(0);
        const SOCK_TOKEN: mio::Token = mio::Token(1);
        let poll = mio::Poll::new().unwrap();
        poll.register(&tunfd, TUN_TOKEN, mio::Ready::readable(), mio::PollOpt::level()).expect("unable register TUN fd");
        poll.register(&sockfd, SOCK_TOKEN, mio::Ready::readable(), mio::PollOpt::level()).expect("unable register SOCK fd");

        let mut events = mio::Events::with_capacity(1024);
        info!("ready transmission");

        loop {
            poll.poll(&mut events, None).expect("poll failed");
            for event in events.iter() {
                match event.token() {
                    SOCK_TOKEN => {
                        let (len,address) = sockfd.recv_from(&mut buf).unwrap();
                        let decrypted_buf_len = receiver.decrypt(&mut buf[..len], &nonce, &add).unwrap();
                        let msg: boring::Message = deserialize(&buf[..decrypted_buf_len]).unwrap();
                        match msg {
                            boring::Message::Request{msg: _} | boring::Message::Response{ip: _,netmask: _,token: _,dns: _} => {
                                warn!("Invalid message {:?} from {}", msg, address);
                            },
                            boring::Message::Data {ip: _,token: recv_token, data} => {
                                if self.token == recv_token {
                                    let data_len = data.len();
                                    let mut sent_len = 0;
                                    while sent_len < data_len {
                                        sent_len += tun.write(&data[sent_len..data_len]).unwrap();
                                    }
                                } else {
                                    warn!("Token mismatched. Received: {}. Expected: {}",recv_token,self.token);
                                }
                            }
                        }
                    },
                    TUN_TOKEN => {
                        let len: usize = tun.read(&mut buf).unwrap();
                        let data = &buf[..len];
                        let msg = boring::Message::Data {
                            ip: self.ip,
                            token: self.token,
                            data: data.to_vec()
                        };
                        let encoded_data = serialize(&msg).unwrap();
                        let mut encrypted_msg = encoded_data.clone();
                        encrypted_msg.resize(encrypted_msg.len() + sender.additional_bytes(), 0);
                        let add = [0u8; 8];
                        let data_len = sender.encrypt(&mut encrypted_msg, encoded_data.len(), &mut nonce, &add);
                        let mut sent_len = 0;
                        while sent_len < data_len {
                            sent_len += sockfd.send_to(&encrypted_msg[sent_len..data_len], &remote_addr).unwrap();
                        }

                    },
                    _ => unreachable!()
                }
            }
        }
        Ok(())
    }

    pub fn connect_tcp() {
        unimplemented!()
    }
}
