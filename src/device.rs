use serde::{Serialize, Deserialize};
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};


const IFNAMESIZE: usize = 16;


extern {
    fn setup_tap_device(fd: i32, ifname: *mut u8) -> i32;
    fn setup_tun_device(fd: i32, ifname: *mut u8) -> i32;
    fn up_device(ifname: *mut u8) -> i32;
    fn set_route(ifname: *mut u8,dst: *mut u8,mask: *mut u8,gateway_addr: *mut u8) -> i32;
    fn set_ip(ifname: *mut u8,ip: *mut u8,netmask: *mut u8) -> i32;
}

// #[derive(Serialize, Deserialize, Debug)]
pub enum Type {
    Tun,
    Tap
}

pub trait Device: AsRawFd {
    fn get_ifname(&self) -> &str;
    fn read(&mut self,buf: &mut [u8]) -> Result<usize,io::Error>;
    fn write(&mut self,buf: &mut [u8]) -> Result<usize,io::Error>;
}

pub struct Tuntap {
    if_fs: fs::File,
    if_name: String,
    type_device: Type,
}

impl Tuntap {
    pub fn create(ifname: &str,type_device: Type,path_device: Option<&path::Path>) -> Result<Tuntap,io::Error> {
        let path_device = path_device.unwrap_or_else(|| path::Path::new("/dev/net/tun"));
        let if_fs = fs::OpenOptions::new().read(true).write(true).open(path_device).expect("open tun failed");
        let name = format!("{}",ifname);
        let mut buf = [0u8;IFNAMESIZE];
        buf[0..name.len()].clone_from_slice(name.as_bytes());
        let result = match type_device {
            Type::Tun => unsafe{ setup_tun_device(if_fs.as_raw_fd(), buf.as_mut_ptr())},
            Type::Tap => unsafe{ setup_tap_device(if_fs.as_raw_fd(), buf.as_mut_ptr())}
        };
        match result {
            0 => {
                let size = buf.iter().position(|&r| r == 0).unwrap();
                Ok(Self{
                    if_fs: if_fs,
                    if_name: String::from_utf8(buf[..size].to_vec()).unwrap(),
                    type_device: type_device,
                })
            },
            _ => Err(io::Error::last_os_error())
        }
    }
    pub fn up(&self) -> Result<(),io::Error>{
        let name = format!("{}",self.if_name);
        let mut buf = [0u8;IFNAMESIZE];
        buf[0..name.len()].clone_from_slice(name.as_bytes());
        let err = unsafe{
            up_device(buf.as_mut_ptr())
        };
        match err {
            1 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }
    pub fn set_ip(&self,ip: &mut [u8],netmask: &mut [u8]) -> Result<(),io::Error>{
        let name = format!("{}",self.if_name);
        let mut buf = [0u8;IFNAMESIZE];
        buf[0..name.len()].clone_from_slice(name.as_bytes());
        let ifname = buf.as_mut_ptr();
        let ip_addr = ip.as_mut_ptr();
        let netmask = netmask.as_mut_ptr();
        let err = unsafe {
            set_ip(ifname, ip_addr, netmask)
        };
        match err {
            1 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }
}

pub fn ip_route(ifname: &mut [u8],dst: &mut [u8],mask: &mut [u8],gateway_addr: &mut [u8]) -> Result<(),io::Error>{
    let ifname = ifname.as_mut_ptr();
    let dst = dst.as_mut_ptr();
    let netmask = mask.as_mut_ptr();
    let gateway_addr = gateway_addr.as_mut_ptr();
    let err = unsafe {
        set_route(ifname, dst,netmask,gateway_addr)
    };
    match err {
        1 => Ok(()),
        _ => Err(io::Error::last_os_error())
    }
}