use serde::{Serialize, Deserialize};
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::io::{Write, Read};
use std::ffi::CString;
use std::os::raw::c_char;

const IFNAMESIZE: usize = 16;


extern {
    fn setup_tap_device(fd: i32, ifname: *mut u8) -> i32;
    fn setup_tun_device(fd: i32, ifname: *mut u8) -> i32;
    fn up_device(ifname: *mut u8) -> i32;
    fn set_ip(ifname: *mut u8,ip: *const c_char,netmask: *const c_char) -> i32;
}

// #[derive(Serialize, Deserialize, Debug)]
pub enum Type {
    Tun,
    Tap
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
    pub fn ifname(&self) -> String {
        self.if_name.clone()
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
    pub fn set_ip(&self,ip: &str,netmask: &str) -> Result<(),io::Error>{
        let name = format!("{}",self.if_name);
        let mut buf = [0u8;IFNAMESIZE];
        buf[0..name.len()].clone_from_slice(name.as_bytes());
        let mut ifname = buf;
        let ip_addr = CString::new(ip).expect("Cstring failed");
        let netmask = CString::new(netmask).expect("Cstring failed");
        let err = unsafe {
            set_ip(ifname.as_mut_ptr(), ip_addr.as_ptr(), netmask.as_ptr())
        };
        match err {
            1 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }
}


impl AsRawFd for Tuntap {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.if_fs.as_raw_fd()
    }
}

impl Read for Tuntap {
    fn read(&mut self,buf: &mut [u8]) -> Result<usize,io::Error> {
        self.if_fs.read(buf)
    }
}

impl Write for Tuntap {
    fn write(&mut self,buf: &[u8]) -> Result<usize,io::Error> {
        self.if_fs.write(buf)
    }
    fn flush(&mut self) -> Result<(),io::Error> {
        self.if_fs.flush()
    }
}

#[cfg(test)]
mod tests {
    use crate::device::*;
    use crate::utils::*;
    use std::process;
    use std::{thread, time};
    #[test]
    fn create_tun_test() {
        assert!(is_root());
        let tun = Tuntap::create("tun1", Type::Tun, None).unwrap();
        let name = tun.if_name;
        let output = process::Command::new("ifconfig")
            .arg(name)
            .output()
            .expect("failed to create tun device");
        assert!(output.status.success());
    }
    #[test]
    fn set_ip_test() {
        assert!(is_root());
        let tun = Tuntap::create("tun2", Type::Tun, None).unwrap();
        let ip = format!("{}","192.168.1.2");
        let netmask = format!("{}","255.255.255.0");
        tun.set_ip(&ip,&netmask).unwrap();
        let name = tun.ifname();
        let output = process::Command::new("ifconfig")
            .arg(name)
            .output()
            .expect("failed to create tun device");
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("192.168.1.2"));
    }
}