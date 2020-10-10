use crate::error::err_exit;
use anyhow::Result;
use clap::App;
use clap::Arg;
use clap::ArgGroup;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;

pub struct Config {
    pub daemon: bool,
    pub conf: Conf,
}

#[derive(Deserialize)]
pub struct Conf {
    pub server: Server,
    pub portmap: Vec<PortMap>,
}

#[derive(Deserialize)]
pub struct Server {
    pub proto: String,
    pub addr: String,
    pub client: String,
}

#[derive(Deserialize)]
pub struct PortMap {
    pub sproto: String,
    pub port: String,
    pub dproto: String,
    pub addr: String,
}

pub static CONFIG: Lazy<Config> = Lazy::new(parse_config);

fn command_config() -> App<'static, 'static> {
    App::new("Shadow Peer Server")
        .name(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("PATH")
                .help("Load config file at PATH")
                .takes_value(true)
                .multiple(false)
                .required(false),
        )
        .arg(
            Arg::with_name("daemon")
                .short("d")
                .long("daemon")
                .help("Start in daemon mode")
                .takes_value(false)
                .multiple(false)
                .required(false),
        )
        .arg(
            Arg::with_name("dump config")
                .short("D")
                .long("dump-config")
                .help("Provide a default sample config")
                .takes_value(false)
                .multiple(false)
                .required(false),
        )
        .group(
            ArgGroup::with_name("config group")
                .args(&["config", "dump config"])
                .multiple(false),
        )
}

fn parse_config() -> Config {
    match parse_config_impl() {
        Ok(config) => config,
        Err(e) => err_exit(1, &format!("init error {}", e)),
    }
}

fn parse_config_impl() -> Result<Config> {
    let matches = command_config().get_matches();

    if matches.is_present("dump config") {
        dump_config();
    }
    let conf = if let Some(conf) = matches.value_of("config") {
        let conf = File::open(conf)?;
        let mut conf = BufReader::new(conf);
        let mut content = String::new();
        conf.read_to_string(&mut content)?;
        toml::from_str(&content)?
    } else {
        toml::from_str(SAMPLE)?
    };
    let daemon = matches.is_present("daemon");

    Ok(Config { daemon, conf })
}

const SAMPLE: &str = r#"[server]
proto = "tcp"
addr = "[::1]:32767"
client = "BITCOINCASH:QPZNZ089TQKAVWF6XM6SD8KPGM59FF5H6CKV0585EP"

[[portmap]]
sproto = "tcp"
port = "8000"
dproto = "tcp"
addr = "[::1]:80"

[[portmap]]
sproto = "tcp"
port = "8443"
dproto = "tcp"
addr = "[::1]:443"

# Save this as an .toml file."#;

fn dump_config() -> ! {
    println!("{}", SAMPLE);
    std::process::exit(0)
}
