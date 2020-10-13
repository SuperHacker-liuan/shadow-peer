use super::Client;
use super::ClientMap;
use super::ReqMapMessage;
use super::ReqMapSender;
use super::ReqStat;
use crate::error::err_exit;
use crate::error::Error;
use crate::error::FastResult;
use crate::error::Result;
use crate::protocol::net_proto::Establish;
use crate::protocol::net_proto::TcpEstablish;
use crate::protocol::ClientId;
use crate::protocol::Protocol;
use async_std::future::timeout;
use async_std::io;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::stream::StreamExt;
use async_std::task;
use futures::channel::oneshot;
use futures::channel::oneshot::Receiver;
use futures::future::FutureExt;
use log::warn;
use std::net::SocketAddr;
use std::time::Duration;

pub(in crate::server) async fn tcp(
    listen: SocketAddr,
    id: ClientId,
    cli: ClientMap,
    req: ReqMapSender,
) -> Result<()> {
    let port = listen.port() as u32;
    let tcp = TcpListener::bind(listen).await?;
    let mut tcp = tcp.incoming();
    while let Some(stream) = tcp.next().await {
        let stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let _ = tcp_stream(stream, id.clone(), &cli, &req).await;
    }
    err_exit(65, Error::ListenFail("TCP", port))
}

struct StreamWaitor<'a> {
    req: &'a ReqMapSender,
    establish: Establish,
    recv: Option<Receiver<TcpStream>>,
}

impl<'a> StreamWaitor<'a> {
    fn register(req: &'a ReqMapSender, est: Establish, cli: &Client) -> FastResult<Self> {
        let (send, recv) = oneshot::channel();
        let stat = ReqStat::Syn(send);
        let msg = ReqMapMessage::Set(stat);
        let protocol = Protocol::Establish(est.clone());
        req.unbounded_send((est.clone(), msg))?;
        cli.estab_sender.unbounded_send(protocol)?;
        Ok(StreamWaitor {
            req,
            establish: est,
            recv: Some(recv),
        })
    }

    async fn recv(&mut self) -> Option<TcpStream> {
        const TMOUT: u64 = 10;
        let mut recv = None;
        std::mem::swap(&mut recv, &mut self.recv);
        let recv = match recv {
            Some(r) => r,
            None => return None,
        };
        match timeout(Duration::from_secs(TMOUT), recv).await {
            Ok(Ok(stream)) => Some(stream),
            _ => None, // Canceled / Timeout
        }
    }
}

impl<'a> Drop for StreamWaitor<'a> {
    fn drop(&mut self) {
        if let Some(_) = self.recv {
            let msg = ReqMapMessage::Unset;
            let _ = self.req.unbounded_send((self.establish.clone(), msg));
        }
    }
}

async fn tcp_stream(
    stream: TcpStream,
    id: ClientId,
    cli: &ClientMap,
    req: &ReqMapSender,
) -> Result<()> {
    let src = stream.peer_addr()?;
    let dest = stream.local_addr()?;
    let establish = TcpEstablish { src, dest };
    let establish = Establish::Tcp(establish);
    let cli = cli.clone();
    let req = req.clone();

    task::spawn(async move {
        let mut sw = match { cli.read().await.get(&id) } {
            Some(cli) => match StreamWaitor::register(&req, establish.clone(), cli) {
                Ok(sw) => sw,
                Err(e) => {
                    warn!(target: "shadow-peer", "{}", e);
                    return;
                }
            },
            None => return,
        };

        // Wait for client connection
        let cli_stream = match sw.recv().await {
            Some(s) => s,
            None => return,
        };

        // Sync
        let (vr, vw) = &mut (&stream, &stream);
        let (cr, cw) = &mut (&cli_stream, &cli_stream);
        futures::select! {
            _ = io::copy(vr, cw).fuse() => {}
            _ = io::copy(cr, vw).fuse() => {}
        }
    });
    Ok(())
}
