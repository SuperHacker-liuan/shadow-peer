use super::ClientMap;
use super::ReqCond;
use super::ReqMap;
use super::ReqStat;
use crate::error::err_exit;
use crate::error::Error;
use crate::error::Result;
use crate::protocol::net_proto::Establish;
use crate::protocol::net_proto::TcpEstablish;
use crate::protocol::ClientId;
use crate::protocol::Protocol;
use async_std::io;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::stream::StreamExt;
use async_std::sync::Arc;
use async_std::sync::Condvar;
use async_std::sync::Mutex;
use async_std::task;
use futures::future::FutureExt;
use std::net::SocketAddr;

pub(in crate::server) async fn tcp(
    listen: SocketAddr,
    id: ClientId,
    cli: ClientMap,
    req: ReqMap,
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

async fn tcp_stream(stream: TcpStream, id: ClientId, cli: &ClientMap, req: &ReqMap) -> Result<()> {
    let src = stream.peer_addr()?;
    let dest = stream.local_addr()?;
    let establish = TcpEstablish { src, dest };
    let establish = Establish::Tcp(establish);
    let cli = cli.clone();
    let req = req.clone();

    task::spawn(async move {
        let resp = match { cli.read().await.get(&id) } {
            Some(cli) => {
                let resp = Arc::new((Mutex::new(None), Condvar::new()));
                let protocol = Protocol::Establish(establish.clone());
                register_on_reqmap(&req, establish, resp.clone()).await;
                cli.estab_sender.send(protocol).await;
                resp
            }
            None => return,
        };

        let (vr, vw) = &mut (&stream, &stream);
        let (lock, cvar) = &*resp;
        let mut cli_stream = lock.lock().await;
        loop {
            cli_stream = cvar.wait(cli_stream).await; //TODO should timeout
            let cli_stream = match *cli_stream {
                Some(ref cli_stream) => cli_stream,
                None => continue,
            };
            let (cr, cw) = &mut (cli_stream, cli_stream);
            let cp1 = io::copy(vr, cw);
            let cp2 = io::copy(cr, vw);
            let _ = futures::select! {
                r1 = cp1.fuse() => r1,
                r2 = cp2.fuse() => r2,
            };
            break;
        }
    });
    Ok(())
}

async fn register_on_reqmap(req: &ReqMap, est: Establish, cond: ReqCond) {
    let stat = ReqStat::Syn(cond);
    req.write().await.insert(est, stat);
}
