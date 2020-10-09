use serde::Deserialize;
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Establish {
    Tcp(TcpEstablish),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TcpEstablish {
    pub src: SocketAddr,
    pub dest: SocketAddr,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Listen {
    Tcp(SocketAddr),
}
