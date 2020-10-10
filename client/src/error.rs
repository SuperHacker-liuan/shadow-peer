use std::fmt::Display;

pub fn err_exit<S: Display>(code: i32, e: S) -> ! {
    eprintln!("{}", e);
    std::process::exit(code)
}
