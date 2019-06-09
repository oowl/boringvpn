use bincode;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::io::{self,Write,Read};
use dns_lookup;
use log::*;
use bincode::{serialize, deserialize};
use std::os::unix::io::AsRawFd;
use mio;
use rand::{thread_rng, Rng};
use transient_hashmap::TransientHashMap;

use crate::device;
use crate::utils;
use crate::boring;
use crate::crypto::{Crypto,CryptoData,CryptoMethod};
use crate::types::Error;

type Token = u64;

#[derive(Debug,Clone)]
pub struct Server {
    ip: IpAddr,
    netmask: IpAddr,
    dns: IpAddr,
    host: IpAddr,
    secret: String,
    port: u16
} 

impl Server {
    pub fn new() -> Self{
        Server {
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            netmask: IpAddr::V4(Ipv4Addr::new(255, 255, 255, 0)),
            dns: IpAddr::V4(Ipv4Addr::new(114, 114, 114, 114)),
            host: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            secret: String::new(),
            port: 0 as u16
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

    fn parse_dns(&mut self,dns: &str) -> Result<(),Error>{
        self.dns = dns.parse().map_err(|e| Error::Parse("failed to parse dns from string",e))?;
        Ok(())
    }

    fn parse_host(&mut self,host: &str) -> Result<(),Error>{
        self.dns = host.parse().map_err(|e| Error::Parse("failed to parse dns from string",e))?;
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
    pub fn server_udp(&mut self) {
        info!("start server");
        info!("server {}:{}",self.host.to_string(),self.port.to_string());
        info!("Enabling kernel's IPv4 forwarding.");
        utils::enable_ipv4_forwarding().unwrap();

        info!("Bringing up TUN device.");
        let mut tun = self.create_tun().unwrap();
        info!("tun device create successful,set ip: {} netmask: {}",self.ip.to_string(),self.netmask.to_string());
        tun.up().unwrap();
        let tun_raw_fd = tun.as_raw_fd();

        let tunfd = mio::unix::EventedFd(&tun_raw_fd);
        info!("TUN device {} initialized. Internal IP: {} {}.",self.ip,self.netmask,tun.ifname());

        let addr = format!("0.0.0.0:{}", self.port.to_string()).parse().unwrap();
        let sockfd = mio::net::UdpSocket::bind(&addr).unwrap();
        info!("Listening on: 0.0.0.0:{}.", self.port);

        let poll = mio::Poll::new().unwrap();
        const TUN_TOKEN: mio::Token = mio::Token(0);
        const SOCK_TOKEN: mio::Token = mio::Token(1);
        poll.register(&sockfd, TUN_TOKEN, mio::Ready::readable(), mio::PollOpt::level()).unwrap();
        poll.register(&tunfd, SOCK_TOKEN, mio::Ready::readable(), mio::PollOpt::level()).unwrap();

        let mut events = mio::Events::with_capacity(1024);
        let mut rng = thread_rng();
        let mut available_ids: Vec<u8> = (2..254).collect();
        let mut client_info: TransientHashMap<IpAddr, (Token, SocketAddr)> = TransientHashMap::new(60);


        let mut buf = [0u8; 1600];
        let mut nonce = [0u8; 12];
        let add = [0u8; 8];
        let mut sender =  Crypto::from_shared_key(CryptoMethod::AES256, &self.secret);
        let receiver = Crypto::from_shared_key(CryptoMethod::AES256, &self.secret);

        loop {
            poll.poll(&mut events, None).expect("poll failed");
            for event in events.iter() {
                match event.token() {
                    SOCK_TOKEN => {
                        let (len,address) = sockfd.recv_from(&mut buf).unwrap();
                        let decrypted_buf_len = receiver.decrypt(&mut buf[..len], &nonce, &add).unwrap();
                        let msg: boring::Message = deserialize(&buf[..decrypted_buf_len]).unwrap();
                        match msg {
                            boring::Message::Request{msg: msg} => {
                                if msg == "hello" {
                                    let client_id: u8 = available_ids.pop().unwrap();
                                    let mut ipaddr_oct = [0u8;4];
                                    match self.ip {
                                        IpAddr::V4(ipv4) => {
                                            ipaddr_oct = ipv4.octets();
                                        },
                                        IpAddr::V6(ipv6) => {},
                                    };
                                    ipaddr_oct[3] = client_id;
                                    let client_ip = Ipv4Addr::new(ipaddr_oct[0], ipaddr_oct[1], ipaddr_oct[2], ipaddr_oct[3]);
                                    let client_token: Token = rng.gen::<Token>();
                                    client_info.insert(IpAddr::V4(client_ip), (client_token, addr));

                                    info!("Got request from {}. Assigning IP address: {}.",
                                      addr,
                                      client_ip.to_string());
                                    let response_msg = boring::Message::Response {
                                        ip: IpAddr::V4(client_ip),
                                        netmask: self.netmask,
                                        token: client_token,
                                        dns: self.dns
                                    };
                                    let encoded_data = serialize(&response_msg).unwrap();
                                    let mut encrypted_msg = encoded_data.clone();
                                    encrypted_msg.resize(encrypted_msg.len() + sender.additional_bytes(), 0);
                                    let add = [0u8; 8];
                                    let data_len = sender.encrypt(&mut encrypted_msg, encoded_data.len(), &mut nonce, &add);
                                    let mut sent_len = 0;
                                    while sent_len < data_len {
                                        sent_len += sockfd.send_to(&encrypted_msg[sent_len..data_len], &address).unwrap();
                                    };
                                } else {
                                    warn!("Invalid message {:?} from {}", msg, address);
                                }
                            },
                            boring::Message::Response{ip: _,netmask: _,token: _,dns: _} => {
                                warn!("Invalid message {:?} from {}", msg, address);
                            },
                            boring::Message::Data {ip,token, data} => {
                                match client_info.get(&ip) {
                                    None => warn!("Unknown data with token {} from ip {}.", token, ip),
                                    Some(&(t, _)) => {
                                        if t != token {
                                            warn!("Unknown data with mismatched token {} from ip {}. \
                                                   Expected: {}",
                                                token,
                                                ip,
                                                t);
                                        } else {
                                            let data_len = data.len();
                                            let mut sent_len = 0;
                                            while sent_len < data_len {
                                                sent_len += tun.write(&data[sent_len..data_len]).unwrap();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    TUN_TOKEN => {
                        let len: usize = tun.read(&mut buf).unwrap();
                        let data = &buf[..len];
                        let client_ip: Vec<u8> = data[16..19].to_vec();
                        let client_ip = IpAddr::V4(Ipv4Addr::new(client_ip[0], client_ip[1], client_ip[2], client_ip[3]));
                        match client_info.get(&client_ip) {
                            None => warn!("Unknown data to ip {}.", client_ip.to_string()),
                            Some(&(token,addr)) => {
                                let msg = boring::Message::Data {
                                    ip: self.ip,
                                    token: token,
                                    data: data.to_vec()
                                };
                                let encoded_data = serialize(&msg).unwrap();
                                let mut encrypted_msg = encoded_data.clone();
                                encrypted_msg.resize(encrypted_msg.len() + sender.additional_bytes(), 0);
                                let add = [0u8; 8];
                                let data_len = sender.encrypt(&mut encrypted_msg, encoded_data.len(), &mut nonce, &add);
                                let mut sent_len = 0;
                                while sent_len < data_len {
                                    sent_len += sockfd.send_to(&encrypted_msg[sent_len..data_len], &addr).unwrap();
                                    }
                                }
                        }
                    },
                    _ => unreachable!()
                }
            }

        }
    }

}