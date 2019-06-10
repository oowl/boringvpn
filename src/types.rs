use std::fmt;
use std::io::{self, Write};
use std::net::AddrParseError;

#[derive(Debug)]
pub enum Error {
    Parse(&'static str,AddrParseError),
    Socket(&'static str, io::Error),
    Name(String),
    TunTapDev(&'static str, io::Error),
    Crypto(&'static str),
    File(&'static str, io::Error),
    Beacon(&'static str, io::Error),
    Shakehand(&'static str,io::Error),
    Invaildmessage(&'static str),
    Route(&'static str)
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::Parse(msg,ref err) => write!(formatter, "{}: {:?}", msg, err),
            Error::Socket(msg, ref err) => write!(formatter, "{}: {:?}", msg, err),
            Error::TunTapDev(msg, ref err) => write!(formatter, "{}: {:?}", msg, err),
            Error::Crypto(msg) => write!(formatter, "{}", msg),
            Error::Name(ref name) => write!(formatter, "failed to resolve name '{}'", name),
            Error::File(msg, ref err) => write!(formatter, "{}: {:?}", msg, err),
            Error::Beacon(msg, ref err) => write!(formatter, "{}: {:?}", msg, err),
            Error::Shakehand(msg,ref err) => write!(formatter, "{}: {:?}", msg, err),
            Error::Invaildmessage(msg) => write!(formatter, "{}", msg),
            Error::Route(msg) => write!(formatter, "{}", msg)
        }
    }
}

