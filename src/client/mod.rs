use crate::error::Error;
use crate::error::Result;
use crate::protocol::net_proto::Establish;
use crate::protocol::net_proto::TcpEstablish;
use crate::protocol::read_protocol;
use crate::protocol::write_protocol;
pub use crate::protocol::ClientId;
use crate::protocol::Protocol;
use crate::protocol::CURRENT_VERSION;
use async_std::io;
use async_std::net::TcpStream;
use async_std::task;
use futures::FutureExt;
use futures_timer::Delay;
use log::warn;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

pub struct Client {
    client_id: ClientId,
    port_map: HashMap<u16, SocketAddr>,
    server: SocketAddr,
}

impl Client {
    pub fn new(
        server: SocketAddr,
        client_id: ClientId,
        port_map: Vec<(u16, SocketAddr)>,
    ) -> Client {
        let port_map = port_map.into_iter().collect();
        Client {
            client_id,
            port_map,
            server,
        }
    }

    pub async fn run(mut self) {
        loop {
            if let Err(e) = self.run_impl().await {
                warn!(target: "shadow-peer", "{}", e);
                Delay::new(Duration::from_secs(3)).await;
            }
        }
    }

    async fn run_impl(&mut self) -> Result<()> {
        let mut ctrl = TcpStream::connect(self.server).await?;
        let ctrl = &mut ctrl;
        let hello = Protocol::ClientId(self.client_id.clone());
        write_wrap(ctrl, &hello).await?;
        loop {
            match read_protocol(ctrl).await? {
                Protocol::Ping(ts) => write_wrap(ctrl, &Protocol::Ping(ts)).await?,
                Protocol::Establish(Establish::Tcp(est)) => {
                    let dest = match self.port_map.get(&est.dest.port()) {
                        Some(dest) => *dest,
                        None => continue, //TODO ignore proto
                    };
                    task::spawn(worker(self.server, dest, est));
                }
                p => return Err(Error::InvalidOperation(format!("{:?}", p))),
            }
        }
    }
}

async fn worker(server: SocketAddr, dest: SocketAddr, est: TcpEstablish) {
    let _ = worker_impl(server, dest, est).await;
}

async fn worker_impl(server: SocketAddr, dest: SocketAddr, est: TcpEstablish) -> Result<()> {
    let dest = TcpStream::connect(dest).await?;
    let mut server = TcpStream::connect(server).await?;
    let hello = Protocol::Establish(Establish::Tcp(est));
    write_wrap(&mut server, &hello).await?;

    // Sync
    let (dr, dw) = &mut (&dest, &dest);
    let (sr, sw) = &mut (&server, &server);
    futures::select! {
        r = io::copy(dr, sw).fuse() => r?,
        r = io::copy(sr, dw).fuse() => r?,
    };
    Ok(())
}

async fn write_wrap(s: &mut TcpStream, proto: &Protocol) -> Result<()> {
    write_protocol(s, CURRENT_VERSION, proto).await
}
