use log::error;
use std::fmt::Display;

pub fn err_exit<S: Display>(code: i32, e: S) -> ! {
    error!(target: "shadow-peer", "{}", e);
    std::process::exit(code)
}
