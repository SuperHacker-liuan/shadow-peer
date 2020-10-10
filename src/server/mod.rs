pub use self::client::CliListen;
use crate::error::err_exit;
use crate::protocol::net_proto::Establish;
pub use crate::protocol::net_proto::Listen;
pub use crate::protocol::ClientId;
use crate::protocol::Protocol;
use async_std::net::TcpStream;
use async_std::sync::Arc;
use async_std::sync::Condvar;
use async_std::sync::Mutex;
use async_std::sync::RwLock;
use async_std::sync::Sender;
use async_std::task;
use std::collections::HashMap;
use std::collections::HashSet;

mod client;
mod visitor;

pub(self) type ClientMap = Arc<RwLock<HashMap<ClientId, Client>>>;
type ReqMap = Arc<Mutex<HashMap<Establish, ReqStat>>>;
type ReqCond = Arc<(Mutex<Option<TcpStream>>, Condvar)>;

pub struct Server {
    cli_listen: Vec<CliListen>,
    client: ClientMap,
    listen: HashMap<Listen, ClientId>,
    request: ReqMap,
    valid_client: Arc<HashSet<ClientId>>,
}

impl Server {
    pub fn new(listen: Vec<(Listen, ClientId)>, cli_listen: Vec<CliListen>) -> Server {
        let valid_client = Arc::new(listen.iter().map(|(_, id)| id.clone()).collect());
        let listen = listen.into_iter().collect();
        Server {
            cli_listen,
            client: Arc::new(RwLock::new(HashMap::new())),
            listen,
            request: Arc::new(Mutex::new(HashMap::new())),
            valid_client,
        }
    }

    pub async fn run(self) {
        let mut join = vec![];
        // Clients Listen
        for listen in self.cli_listen {
            let cli = self.client.clone();
            let reqmap = self.request.clone();
            let idset = self.valid_client.clone();
            let task = async move {
                match listen {
                    CliListen::Tcp(socket) => client::tcp(socket, cli, reqmap, idset).await,
                }
                .unwrap_or_else(|e| err_exit(1, e));
            };
            join.push(task::spawn(task));
        }

        // Visitors Listen
        for (listen, id) in self.listen {
            let climap = self.client.clone();
            let reqmap = self.request.clone();
            let task = async move {
                match listen {
                    Listen::Tcp(socket) => visitor::tcp(socket, id, climap, reqmap).await,
                }
                .unwrap_or_else(|e| err_exit(2, e));
            };
            join.push(task::spawn(task));
        }
        for handle in join {
            handle.await;
        }
    }
}

pub(self) struct Client {
    estab_sender: Sender<Protocol>,
}

enum ReqStat {
    Syn(ReqCond),
}
