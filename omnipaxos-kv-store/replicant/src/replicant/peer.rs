use std::collections::HashMap;
use std::sync::Arc;
use omnipaxos::messages::Message;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use crate::replicant::kvstore::kv::Command;

pub struct Peer {
    pub(crate) id: i64,
    pub(crate) stub: Arc<TcpLink>,
}

async fn handle_outgoing_requests(
    mut write_half: OwnedWriteHalf,
    mut request_recv: UnboundedReceiver<Message<Command>>
) {
    while let Some(mut request) = request_recv.recv().await {
        let mut data = serde_json::to_vec(&request)
            .expect("could not serialize msg");
        data.push(b'\n');
        write_half.write_all(&data).await
            .expect("peer cannot send msg");
    }
}

pub struct TcpLink {
    request_sender: UnboundedSender<Message<Command>>,
}

impl TcpLink {
    pub async fn new(addr: &str) -> Self {
        let stream;
        loop {
            match TcpStream::connect(addr).await {
                Ok(s) => {stream = s; stream.set_nodelay(true); break;},
                _ => (),
            }
        }
        let (read_half, write_half) = stream.into_split();
        let (request_sender, request_recv) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            handle_outgoing_requests(write_half, request_recv).await;
        });
        Self {
            request_sender,
        }
    }

    pub fn send_await_response(&self, msg: Message<Command>) {
        self.request_sender.send(msg).
            expect("cannot send request via channel");
    }
}
