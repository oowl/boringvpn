use libc;

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}