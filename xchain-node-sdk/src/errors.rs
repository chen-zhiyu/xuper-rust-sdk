use serde_derive;
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Error {
    repr: Repr,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.repr, f)
    }
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
enum Repr {
    Simple(ErrorKind),
    #[serde(skip)]
    Custom(Box<Custom>),
}

#[derive(Debug)]
struct Custom {
    kind: ErrorKind,
    error: Box<dyn std::error::Error + Send + Sync>,
}

#[derive(
    serde_derive::Serialize,
    serde_derive::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]

pub enum ErrorKind {
    InvalidArguments = 1,
    ParseError = 2,
    CryptoError = 3,
    ChainRPCError = 4,
    ContractCodeGT400 = 5,
    Unknown,
}

impl ErrorKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ErrorKind::InvalidArguments => "invalid arguments",
            ErrorKind::ParseError => "parsing error",
            ErrorKind::CryptoError => "crypto error",
            ErrorKind::ChainRPCError => "rpc to chain node error",
            ErrorKind::ContractCodeGT400 => "contract invoking return code greater than 400",
            ErrorKind::Unknown => "unknown error",
        }
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Error {
        Error {
            repr: Repr::Simple(kind),
        }
    }
}

impl From<u32> for Error {
    #[inline]
    fn from(kind: u32) -> Error {
        let err_kind = match kind {
            0x0000_0001 => ErrorKind::InvalidArguments,
            0x0000_0002 => ErrorKind::ParseError,
            0x0000_0003 => ErrorKind::CryptoError,
            0x0000_0004 => ErrorKind::ChainRPCError,
            0x0000_0005 => ErrorKind::ContractCodeGT400,
            _ => ErrorKind::Unknown,
        };

        Error {
            repr: Repr::Simple(err_kind),
        }
    }
}

impl From<hex::FromHexError> for Error {
    #[inline]
    fn from(err: hex::FromHexError) -> Error {
        Error::new(ErrorKind::ParseError, err)
    }
}

impl From<grpc::Error> for Error {
    #[inline]
    fn from(err: grpc::Error) -> Error {
        Error::new(ErrorKind::ParseError, err)
    }
}

impl From<serde_json::Error> for Error {
    #[inline]
    fn from(err: serde_json::Error) -> Error {
        Error::new(ErrorKind::ParseError, err)
    }
}

impl From<xchain_crypto::errors::Error> for Error {
    #[inline]
    fn from(err: xchain_crypto::errors::Error) -> Error {
        Error::new(ErrorKind::CryptoError, err)
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(err: std::io::Error) -> Error {
        Error::new(ErrorKind::ParseError, err)
    }
}

impl Into<u32> for Error {
    #[inline]
    fn into(self) -> u32 {
        match self.kind() {
            ErrorKind::InvalidArguments => 0x0000_0001,
            ErrorKind::ParseError => 0x0000_0002,
            ErrorKind::CryptoError => 0x0000_0003,
            ErrorKind::ChainRPCError => 0x0000_0004,
            ErrorKind::ContractCodeGT400 => 0x0000_0005,
            ErrorKind::Unknown => 0xffff_ffff,
        }
    }
}

impl Error {
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::_new(kind, error.into())
    }

    fn _new(kind: ErrorKind, error: Box<dyn std::error::Error + Send + Sync>) -> Error {
        Error {
            repr: Repr::Custom(Box::new(Custom { kind, error })),
        }
    }

    pub fn get_ref(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::Simple(..) => None,
            Repr::Custom(ref c) => Some(&*c.error),
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut (dyn std::error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::Simple(..) => None,
            Repr::Custom(ref mut c) => Some(&mut *c.error),
        }
    }

    pub fn into_inner(self) -> Option<Box<dyn std::error::Error + Send + Sync>> {
        match self.repr {
            Repr::Simple(..) => None,
            Repr::Custom(c) => Some(c.error),
        }
    }

    pub fn into_simple_error(self) -> Error {
        match self.repr {
            Repr::Simple(_) => self,
            Repr::Custom(c) => Error::from(c.kind),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        match self.repr {
            Repr::Custom(ref c) => c.kind,
            Repr::Simple(kind) => kind,
        }
    }

    pub fn unknown() -> Error {
        Error::from(ErrorKind::Unknown)
    }
}

impl fmt::Debug for Repr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Repr::Custom(ref c) => fmt::Debug::fmt(&c, fmt),
            Repr::Simple(kind) => fmt.debug_tuple("Kind").field(&kind).finish(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.repr {
            Repr::Custom(ref c) => c.error.fmt(fmt),
            Repr::Simple(kind) => write!(fmt, "{}", kind.as_str()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.repr {
            Repr::Simple(..) => None,
            Repr::Custom(ref c) => c.error.source(),
        }
    }
}
