pub use self::client::CliListen;
use crate::error::err_exit;
use crate::protocol::net_proto::Establish;
pub use crate::protocol::net_proto::Listen;
use crate::protocol::ClientId;
use crate::protocol::Protocol;
use async_std::net::TcpStream;
use async_std::sync::Arc;
use async_std::sync::Condvar;
use async_std::sync::Mutex;
use async_std::sync::RwLock;
use async_std::sync::Sender;
use async_std::task;
use std::collections::HashMap;

mod client;
mod visitor;

pub(self) type ClientMap = Arc<RwLock<HashMap<ClientId, Client>>>;
type ReqMap = Arc<RwLock<HashMap<Establish, ReqStat>>>;
type ReqCond = Arc<(Mutex<Option<TcpStream>>, Condvar)>;

pub struct Server {
    cli_listen: Vec<CliListen>,
    client: ClientMap,
    listen: HashMap<Listen, ClientId>,
    request: ReqMap,
}

impl Server {
    pub fn new(listen: Vec<(Listen, ClientId)>, cli_listen: Vec<CliListen>) -> Server {
        let listen = listen.into_iter().collect();
        Server {
            cli_listen,
            client: Arc::new(RwLock::new(HashMap::new())),
            listen,
            request: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self) {
        // Clients Listen
        for listen in self.cli_listen {
            let cli = self.client.clone();
            let reqmap = self.request.clone();
            let task = async move {
                match listen {
                    CliListen::Tcp(socket) => client::tcp(socket, cli, reqmap).await,
                }
                .unwrap_or_else(|e| err_exit(1, e))
            };
            task::spawn(task);
        }
        // Visitors Listen
        for (listen, id) in self.listen {
            let climap = self.client.clone();
            let reqmap = self.request.clone();
            let task = async move {
                match listen {
                    Listen::Tcp(socket) => visitor::tcp(socket, id, climap, reqmap).await,
                }
                .unwrap_or_else(|e| err_exit(2, e))
            };
            task::spawn(task);
        }
    }
}

pub(self) struct Client {
    estab_sender: Sender<Protocol>,
}

enum ReqStat {
    Syn(ReqCond),
}
