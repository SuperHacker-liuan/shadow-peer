use self::config::CONFIG;
use self::error::err_exit;
use anyhow::anyhow;
use anyhow::Result;
use shadow_peer::client::Client;
use shadow_peer::client::ClientId;
use std::net::SocketAddr;
use daemonize::Daemonize;

mod config;
mod error;

#[async_std::main]
async fn main() -> Result<()> {
    let server = parse_server()?;
    let client_id = ClientId::from(&CONFIG.conf.server.client);
    let port_map = CONFIG.conf.portmap.iter().map(port_map_mapper).collect();
    daemonize();
    Client::new(server, client_id, port_map).run().await;
    Ok(())
}

fn daemonize() {
    if !CONFIG.daemon {
        return;
    }
    Daemonize::new()
        .pid_file(format!("/tmp/shadow-peer-client.pid"))
        .working_directory("/tmp")
        .umask(0o777)
        .start()
        .expect("Failed to start as daemon");
}

fn parse_server() -> Result<SocketAddr> {
    let conf = &CONFIG.conf.server;
    match conf.proto.as_ref() {
        "tcp" => Ok(conf.addr.parse()?),
        proto => Err(anyhow!("Unsupported protocol {}", proto)),
    }
}

fn port_map_mapper(pm: &config::PortMap) -> (u16, SocketAddr) {
    match port_map_mapper_impl(pm) {
        Ok(r) => r,
        Err(e) => err_exit(1, e),
    }
}

fn port_map_mapper_impl(pm: &config::PortMap) -> Result<(u16, SocketAddr)> {
    let port = match pm.sproto.as_ref() {
        "tcp" => pm.port.parse()?,
        proto => Err(anyhow!("Unsupported protocol {}", proto))?,
    };

    let addr = match pm.dproto.as_ref() {
        "tcp" => pm.addr.parse()?,
        proto => Err(anyhow!("Unsupported protocol {}", proto))?,
    };

    Ok((port, addr))
}
