use super::Client;
use super::ClientMap;
use super::ReqMap;
use super::ReqStat;
use crate::error::err_exit;
use crate::error::Error;
use crate::error::Result;
use crate::protocol::net_proto::Establish;
use crate::protocol::read_protocol_timeout;
use crate::protocol::write_protocol;
use crate::protocol::ClientId;
use crate::protocol::Protocol;
use crate::protocol::CURRENT_VERSION;
use crate::utils::current_time16;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::stream::StreamExt;
use async_std::sync;
use async_std::sync::Arc;
use async_std::sync::Receiver;
use async_std::task;
use futures::FutureExt;
use futures_timer::Delay;
use std::collections::HashSet;
use std::io;
use std::net::SocketAddr;
use std::result::Result as StdResult;
use std::time::Duration;

struct StreamShare {
    cli: ClientMap,
    idset: Arc<HashSet<ClientId>>,
    req: ReqMap,
}

pub(in crate::server) async fn tcp(
    socket: SocketAddr,
    cli: ClientMap,
    req: ReqMap,
    idset: Arc<HashSet<ClientId>>,
) -> Result<()> {
    let port = socket.port() as u32;
    let tcp = TcpListener::bind(socket).await?;
    let mut tcp = tcp.incoming();
    let share = Arc::new(StreamShare { cli, idset, req });
    while let Some(stream) = tcp.next().await {
        let share = share.clone();
        task::spawn(async move {
            match init(&share, stream).await {
                Some(ConnInit::Control(tcp, recv, id)) => {
                    controller(tcp, recv).await;
                    share.cli.write().await.remove(&id);
                }
                Some(ConnInit::Worker(tcp, est)) => worker(tcp, est, &share.req).await,
                None => {}
            };
        });
    }
    err_exit(66, Error::ListenFail("TCP", port))
}

struct Controller {
    stream: Arc<TcpStream>,
    last_recv: u16,
}

enum ConnInit {
    Control(Controller, Receiver<Protocol>, ClientId),
    Worker(TcpStream, Establish),
}

async fn init(share: &StreamShare, stream: StdResult<TcpStream, io::Error>) -> Option<ConnInit> {
    let mut stream = stream.ok()?;
    let r = match read_protocol_timeout(&mut stream, 10).await.ok()? {
        Protocol::ClientId(id) => {
            let id = ClientId::from(id);
            if !share.idset.contains(&id) {
                return None;
            }
            let (send, recv) = sync::channel(1000);
            let client = Client { estab_sender: send };
            let controller = Controller {
                stream: Arc::new(stream),
                last_recv: current_time16(),
            };
            share.cli.write().await.insert(id.clone(), client);
            ConnInit::Control(controller, recv, id)
        }
        Protocol::Establish(est) => ConnInit::Worker(stream, est),
        _ => return None,
    };
    Some(r)
}

async fn controller(mut c: Controller, recv: Receiver<Protocol>) {
    const PING_TMOUT: u64 = 5;
    let mut send_fut = Box::pin(recv.recv().fuse());
    let mut recv_fut = Box::pin(read_wrap(c.stream.clone()).fuse());
    let mut ping_timer = Delay::new(Duration::from_secs(PING_TMOUT)).fuse();
    loop {
        futures::select! {
            send = send_fut => {
                let proto = match send {
                    Ok(proto) => proto,
                    Err(_) => return,
                };
                if !write_wrap(&mut &*c.stream, &proto).await {
                    return;
                }
                send_fut = Box::pin(recv.recv().fuse());
            },
            recv = recv_fut => {
                let proto = match recv {
                    Ok(proto) => proto,
                    Err(_) => return,
                };
                handle_recv(&mut c, proto);
                recv_fut = Box::pin(read_wrap(c.stream.clone()).fuse());
            },
            _ = ping_timer => {
                let proto = Protocol::Ping(current_time16());
                if !write_wrap(&mut &*c.stream, &proto).await {
                    return;
                }
                ping_timer = Delay::new(Duration::from_secs(PING_TMOUT)).fuse();
            },
        }
    }
}

fn handle_recv(c: &mut Controller, proto: Protocol) -> bool {
    c.last_recv = current_time16();
    match proto {
        Protocol::Ping(_) => true,
        _ => false,
    }
}

async fn worker(stream: TcpStream, est: Establish, req: &ReqMap) {
    let mut map = req.lock().await;
    let stat = match map.remove(&est) {
        Some(ReqStat::Syn(stat)) => stat,
        None => return,
    };

    let (lock, cond) = &*stat;
    *lock.lock().await = Some(stream);
    cond.notify_one();
}

async fn write_wrap(s: &mut &TcpStream, proto: &Protocol) -> bool {
    write_protocol(s, CURRENT_VERSION, proto).await.is_ok()
}

async fn read_wrap(s: Arc<TcpStream>) -> Result<Protocol> {
    read_protocol_timeout(&mut &*s, 10).await
}
