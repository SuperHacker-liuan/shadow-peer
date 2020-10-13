pub use self::client::CliListen;
use self::reqmap::ReqMapMessage;
use crate::error::err_exit;
use crate::protocol::net_proto::Establish;
pub use crate::protocol::net_proto::Listen;
pub use crate::protocol::ClientId;
use crate::protocol::Protocol;
use async_std::sync::Arc;
use async_std::sync::RwLock;
use async_std::task;
use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedSender;
use std::collections::HashMap;
use std::collections::HashSet;

mod client;
mod reqmap;
mod visitor;

pub(self) type ClientMap = Arc<RwLock<HashMap<ClientId, Client>>>;
type ReqMapSender = UnboundedSender<(Establish, ReqMapMessage)>;

pub struct Server {
    cli_listen: Vec<CliListen>,
    client: ClientMap,
    listen: HashMap<Listen, ClientId>,
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
            valid_client,
        }
    }

    pub async fn run(self) {
        let mut join = vec![];
        let (send, recv) = mpsc::unbounded();
        task::spawn(reqmap::actor(recv));
        // Clients Listen
        for listen in self.cli_listen {
            let cli = self.client.clone();
            let send = send.clone();
            let idset = self.valid_client.clone();
            let task = async move {
                match listen {
                    CliListen::Tcp(socket) => client::tcp(socket, cli, send, idset).await,
                }
                .unwrap_or_else(|e| err_exit(1, e));
            };
            join.push(task::spawn(task));
        }

        // Visitors Listen
        for (listen, id) in self.listen {
            let climap = self.client.clone();
            let send = send.clone();
            let task = async move {
                match listen {
                    Listen::Tcp(socket) => visitor::tcp(socket, id, climap, send).await,
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
    estab_sender: UnboundedSender<Protocol>,
}
