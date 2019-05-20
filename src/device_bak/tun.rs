use std::{fs,process,io};
use libc;
use std::path;
use std::os::unix::io::{RawFd, AsRawFd};

const MTU: &'static str = "1400";
const IFNAMESIZE: usize = 16;
const IFF_TUN: libc::c_short = 0x0001;
const IFF_NO_PI: libc::c_short = 0x1000;
const TUNSETIFF: libc::c_ulong = 0x400454ca; 

#[repr(C)]
pub struct ifreq {
    pub ifr_name: [u8;IFNAMESIZE],
    pub ifr_flag: libc::c_short,
}

pub struct Tun {
    tun_fs: fs::File,
    tun_name: String
}

impl Tun {
    pub fn creat(id: u8) -> Result<Tun,io::Error> {
        let tun_path = path::Path::new("/dev/net/tun");
        let tun_fs = fs::OpenOptions::new().read(true).write(true).open(tun_path).expect("open tun failed");
        let mut req = ifreq {
            ifr_name: {
                let name = format!("tun{}",id);
                let mut buf = [0u8;IFNAMESIZE];
                buf[0..name.len()].clone_from_slice(name.as_bytes());
                buf
            },
            ifr_flag: IFF_TUN | IFF_NO_PI
        };
        let err = unsafe { libc::ioctl(tun_fs.as_raw_fd(),TUNSETIFF,&mut req)};
        if err < 0 {
            return Err(io::Error::last_os_error());
        }
        let size = req.ifr_name.iter().position(|&r| r == 0).unwrap();
        let tun = Tun {
            tun_fs: tun_fs,
            tun_name: String::from_utf8(req.ifr_name[..size].to_vec()).unwrap()
        };
        Ok(tun)
    }


}