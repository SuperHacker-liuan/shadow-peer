use crate::protocol::net_proto::Establish;
use async_std::net::TcpStream;
use async_std::stream::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::oneshot;
use std::collections::HashMap;

pub enum ReqMapMessage {
    Set(ReqStat),
    Take(oneshot::Sender<Option<ReqStat>>),
    Unset,
}

pub enum ReqStat {
    Syn(oneshot::Sender<TcpStream>),
}

pub async fn actor(mut events: UnboundedReceiver<(Establish, ReqMapMessage)>) {
    type Msg = ReqMapMessage;
    let mut map = HashMap::new();

    while let Some((est, msg)) = events.next().await {
        match msg {
            Msg::Set(stat) => {
                map.insert(est, stat);
            }
            Msg::Take(send) => {
                let _ = send.send(map.remove(&est));
            }
            Msg::Unset => {
                map.remove(&est);
            }
        }
    }
}
