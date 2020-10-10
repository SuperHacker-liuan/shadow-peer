use self::config::CONFIG;
use self::error::err_exit;
use anyhow::anyhow;
use anyhow::Result;
use daemonize::Daemonize;
use shadow_peer::server::CliListen;
use shadow_peer::server::ClientId;
use shadow_peer::server::Listen;
use shadow_peer::server::Server;

mod config;
mod error;
mod log;

#[async_std::main]
async fn main() {
    let listen = CONFIG.conf.listen.iter().map(listen_mapper).collect();
    let cli = CONFIG.conf.client.iter().map(cli_mapper).collect();
    log::init_logger();
    daemonize();
    Server::new(listen, cli).run().await
}

fn daemonize() {
    if !CONFIG.daemon {
        return;
    }
    Daemonize::new()
        .pid_file(format!("/tmp/shadow-peer-server.pid"))
        .working_directory("/tmp")
        .umask(0o777)
        .start()
        .expect("Failed to start as daemon");
}

fn cli_mapper(c: &config::Client) -> CliListen {
    match cli_mapper_impl(c) {
        Ok(r) => r,
        Err(e) => err_exit(1, e),
    }
}

fn cli_mapper_impl(c: &config::Client) -> Result<CliListen> {
    match c.proto.as_ref() {
        "tcp" => Ok(CliListen::Tcp(c.listen.parse()?)),
        proto => Err(anyhow!("Unsupported protocol {}", proto)),
    }
}

fn listen_mapper(l: &config::Listen) -> (Listen, ClientId) {
    match listen_mapper_impl(l) {
        Ok(r) => r,
        Err(e) => err_exit(1, e),
    }
}

fn listen_mapper_impl(l: &config::Listen) -> Result<(Listen, ClientId)> {
    match l.proto.as_ref() {
        "tcp" => Ok((Listen::Tcp(l.listen.parse()?), ClientId::from(&l.client))),
        proto => Err(anyhow!("Unsupported protocol {}", proto)),
    }
}
