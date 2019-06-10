use clap::{App, Arg, Error,SubCommand};
use clap;
use std::{ffi::OsString,path::PathBuf};
use std::net::{IpAddr,Ipv4Addr,Ipv6Addr};

use crate::client::Client;
use crate::server::Server;


#[derive(Debug,Clone)]
pub enum Args {
    Client(Client),
    Server(Server)
}

pub fn get_args() -> Result<Args,String> {
    let matches = App::new("boringvpn: so boring vpn power by rust")
                            .version("0.1")
                            .subcommand(SubCommand::with_name("server")
                                        .about("server mode")
                                        .version("0.1")
                                        .author("Attenuation <ouyangjun1999@gmail.com>")
                                        .arg(Arg::with_name("bind")
                                            .short("l")
                                            .long("listen")
                                            .default_value("0.0.0.0")
                                            .help("set the listen address")
                                            .takes_value(true))
                                        .arg(Arg::with_name("port")
                                            .short("p")
                                            .long("port")
                                            .default_value("9527")
                                            .help("set the listen port")
                                            .takes_value(true))
                                        .arg(Arg::with_name("key")
                                            .short("k")
                                            .long("key")
                                            .help("set the key for encryption communication")
                                            .takes_value(true))
                                        .arg(Arg::with_name("dns")
                                            .short("d")
                                            .long("dns")
                                            .default_value("8.8.8.8")
                                            .help("set dns for client, default 8.8.8.8")
                                            .takes_value(true))
                                        .arg(Arg::with_name("ip")
                                            .short("i")
                                            .long("ip")
                                            .default_value("10.10.10.1")
                                            .help("set tun ip address")
                                            .takes_value(true))
                                        .arg(Arg::with_name("netmask")
                                            .short("n")
                                            .long("netmask")
                                            .default_value("255.255.255.0")
                                            .takes_value(true)
                                            .help("set tun netmask"))
                            )
                            .subcommand(SubCommand::with_name("client")
                                        .about("client mode")
                                        .version("0.1")
                                        .author("Attenuation <ouyangjun1999@gmail.com>")
                                        .arg(Arg::with_name("server")
                                            .short("s")
                                            .long("server")
                                            .help("set the remote server address")
                                            .takes_value(true))
                                        .arg(Arg::with_name("port")
                                            .short("p")
                                            .long("port")
                                            .help("set the remote port")
                                            .takes_value(true))
                                        .arg(Arg::with_name("key")
                                            .short("k")
                                            .long("key")
                                            .help("set the key for encryption communication")
                                            .takes_value(true))
                                        .arg(Arg::with_name("no-default-route")
                                            .short("n")
                                            .long("no-default-route")
                                            .help("do not set default route"))
                            ).get_matches();
    if let Some(matches) = matches.subcommand_matches("client"){ 
        let ip_str = matches.value_of("server").ok_or_else(|| "can not find client host value").unwrap();
        let port_str = matches.value_of("port").ok_or_else(|| "can not find client port value").unwrap();
        let key_str = matches.value_of("key").ok_or_else(|| "can not find client key value").unwrap();
        // let remote_addr = IpAddr::V4(Ipv4Addr::from_str(ip_str).map_err(|e| e.to_string())?);
        let port = port_str.parse::<u16>().map_err(|e| e.to_string())?;
        let default_route = match matches.is_present("no-default-route"){
            false => true,
            true => false
        };
        let mut client = Client::new();
        client.parse_host(ip_str).unwrap();
        client.parse_port(port);
        client.parse_key(key_str);
        client.parse_default_route(default_route);
        Ok(Args::Client(client))
    } else if let Some(matches) = matches.subcommand_matches("server") {
        let ip_str = matches.value_of("bind").ok_or_else(|| "can not find server host value").unwrap();
        let port_str = matches.value_of("port").ok_or_else(|| "can not find server port value").unwrap();
        let port = port_str.parse::<u16>().map_err(|e| e.to_string())?;
        let key_str = matches.value_of("key").ok_or_else(|| "can not find server key value").unwrap();
        let dns = matches.value_of("dns").ok_or_else(|| "can not find dns value")?;
        let ip = matches.value_of("ip").ok_or_else(|| "can not find ip value")?;
        let netmask = matches.value_of("netmask").ok_or_else(|| "can not find netmask value")?;
        let mut server = Server::new();
        server.parse_host(ip_str).unwrap();
        server.parse_port(port);
        server.parse_dns(dns).unwrap();
        server.parse_key(key_str);
        server.parse_ip(ip).unwrap();
        server.parse_netmask(netmask).unwrap();
        // let bind_addr = IpAddr::V4(Ipv4Addr::from_str(ip_str).map_err(|e| e.to_string())?);
        Ok(Args::Server(server))
    } else {
        unimplemented!()
    }
}