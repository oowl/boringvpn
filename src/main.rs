use serde::{Serialize, Deserialize};
use env_logger;
use libc;

mod utils;
mod crypto;
mod device;
mod types;
mod boring;
mod client;
mod server;
mod cli;

fn main() {
    env_logger::init();
    if !utils::is_root() {
        panic!("Please run as root");
    }
    match cli::get_args().unwrap() {
        cli::Args::Client(mut client) => client.connect_udp().unwrap(),
        cli::Args::Server(mut server) => server.server_udp().unwrap()
    }
    println!("Hello, world!");
}
