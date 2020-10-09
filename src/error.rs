use std::fmt::Display;
use thiserror::Error;

pub type Error = ShadowPeerError;
pub type Result<T> = std::result::Result<T, Error>;

type TimeoutError = async_std::future::TimeoutError;

#[derive(Debug, Error)]
pub enum ShadowPeerError {
    #[error("io")]
    Io(#[from] std::io::Error),
    #[error("invalid operation")]
    InvalidOperation(String),
    #[error("Unexpect listen fail on {0} port {1}")]
    ListenFail(&'static str, u32),
    #[error("serde json")]
    SerdeJson(#[from] serde_json::Error),
    #[error("timeout")]
    Timeout(#[from] TimeoutError),
    #[error("unsupported version {0}")]
    UnsupportedVersion(u8),
}

pub(crate) fn err_exit<S: Display>(code: i32, e: S) -> ! {
    eprintln!("{}", e);
    std::process::exit(code)
}