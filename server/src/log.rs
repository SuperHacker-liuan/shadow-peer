use crate::CONFIG;
use simplelog::CombinedLogger;
use simplelog::LevelFilter;
use simplelog::SharedLogger;
use simplelog::TermLogger;
use simplelog::TerminalMode;
use simplelog::WriteLogger;
use std::fs::File;

pub fn init_logger() {
    let term = TermLogger::new(LevelFilter::Debug, config(), TerminalMode::Mixed);
    let mut logger: Vec<Box<dyn SharedLogger>> = vec![term];
    if let Some(ref path) = CONFIG.log {
        let file = File::create(&path).expect(&format!("Unable to create {:?}", &path));
        let writer = WriteLogger::new(LevelFilter::Info, config(), file);
        logger.push(writer);
    }
    CombinedLogger::init(logger).expect("Failed to init logger");
}

fn config() -> simplelog::Config {
    simplelog::ConfigBuilder::new()
        .set_time_format_str("%F %T")
        .build()
}
