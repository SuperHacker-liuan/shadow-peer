use std::time::SystemTime;

pub fn current_time16() -> u16 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(dur) => dur.as_secs() as u16,
        Err(_) => 0,
    }
}
