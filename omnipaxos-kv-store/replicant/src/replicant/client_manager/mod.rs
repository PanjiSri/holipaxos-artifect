use parking_lot::Mutex;
use tokio::sync::{RwLock};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use log::info;
use omnipaxos::messages::Message as OPMessage;
use omnipaxos::ProposeErr::{ConfigError, PendingReconfigConfig, PendingReconfigEntry};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use crate::replicant::kvstore::kv::{Command, KeyValue};
use crate::replicant::OmniPaxosKV;

pub enum Message {
    OmniPaxosMsg(OPMessage<Command>),
    ClientRequest(Command),
}

fn parse(line: &str, cid: i64) -> Option<Command> {
    let tokens: Vec<&str> = line.trim().split(' ').collect();
    if tokens.len() == 2 {
        if tokens[0] == "get" {
            return Some(Command::Get(KeyValue{
                key: tokens[1].to_string(),
                value: "".to_string(),
                client_id: cid,
            }));
        }
        if tokens[0] == "del" {
            return Some(Command::Delete(KeyValue{
                key: tokens[1].to_string(),
                value: "".to_string(),
                client_id: cid,
            }));
        }
        return None;
    }
    if tokens.len() == 3 && tokens[0] == "put" {
        return Some(Command::Put(KeyValue{
            key: tokens[1].to_string(),
            value: tokens[2].to_string(),
            client_id: cid,
        }));
    }
    None
}

struct Client {
    read_half: BufReader<OwnedReadHalf>,
    client: Arc<ClientInner>,
}

impl Client {
    fn new(
        id: i64,
        read_half: OwnedReadHalf,
        tx: UnboundedSender<Message>,
        manager: Arc<ClientManagerInner>,
        is_from_client: bool,
    ) -> Self {
        Self {
            read_half: BufReader::new(read_half),
            client: Arc::new(ClientInner::new(id, tx, manager, is_from_client)),
        }
    }

    async fn start(&mut self) {
        let mut line = String::new();
        while let Ok(n) = self.read_half.read_line(&mut line).await {
            if n == 0 {
                break;
            }
            if let Some(response) = self.client.handle_request(&line).await {
                self.client.manager.write(self.client.id, response).await;
            }
            line.clear();
        }
        self.client.manager.stop(self.client.id);
    }
}

struct ClientInner {
    id: i64,
    tx: UnboundedSender<Message>,
    manager: Arc<ClientManagerInner>,
    is_from_client: bool,
}

impl ClientInner {
    fn new(
        id: i64,
        tx: UnboundedSender<Message>,
        manager: Arc<ClientManagerInner>,
        is_from_client: bool,
    ) -> Self {
        Self {
            id,
            tx,
            manager,
            is_from_client,
        }
    }

    async fn handle_request(&self, line: &str) -> Option<String> {
        if self.is_from_client {
            self.handle_client_request(line).await
        } else {
            self.handle_peer_request(line).await;
            None
        }
    }

    async fn handle_client_request(&self, line: &str) -> Option<String> {
        if let Some(command) = parse(&line, self.id) {
            let msg: Message = Message::ClientRequest(command);
            self.tx.send(msg).expect("client send channel not working");
            None
        } else {
            Some("bad command".to_string())
        }
    }

    async fn handle_peer_request(&self, line: &str) {
        let msg: OPMessage<Command> = match serde_json::from_str(line) {
            Ok(request) => request,
            Err(_) => return
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tx.send(Message::OmniPaxosMsg(msg));
        });
    }
}

struct ClientManagerInner {
    next_id: AtomicI64,
    num_peers: i64,
    clients: Mutex<HashMap<i64, Arc<tokio::sync::Mutex<OwnedWriteHalf>>>>,
}

impl ClientManagerInner {
    fn new(id: i64, num_peers: i64) -> Self {
        Self {
            next_id: AtomicI64::new(id),
            num_peers,
            clients: Mutex::new(HashMap::new()),
        }
    }

    fn next_client_id(&self) -> i64 {
        self.next_id.fetch_add(self.num_peers, Ordering::Relaxed)
    }

    async fn write(&self, client_id: i64, mut buf: String) {
        let client;
        {
            client = if let Some(client) = self.clients.lock().get(&client_id) {
                Some(client.clone())
            } else {
                None
            }
        }

        buf.push('\n');
        if let Some(client) = client {
            let mut client = client.lock().await;
            match client.write(buf.as_bytes()).await {
                Ok(_) => (),
                Err(_) => self.stop(client_id),
            }
        }
    }

    fn stop(&self, id: i64) {
        let mut clients = self.clients.lock();
        let client = clients.remove(&id);
        drop(clients);
        assert!(client.is_some());
        info!("client_manager stopped client {}", id);
    }

    fn stop_all(&self) {
        let mut clients = self.clients.lock();
        clients.clear();
    }
}

pub struct ClientManager {
    client_manager: Arc<ClientManagerInner>,
    tx: UnboundedSender<Message>,
    is_from_client: bool,
}

impl ClientManager {
    pub fn new(
        id: i64,
        num_peers: i64,
        tx: UnboundedSender<Message>,
        is_from_client: bool
    ) -> Self {
        Self {
            client_manager: Arc::new(ClientManagerInner::new(id, num_peers)),
            tx,
            is_from_client,
        }
    }

    pub fn start(&self, stream: TcpStream) {
        stream.set_nodelay(true);
        let (read_half, write_half) = stream.into_split();
        let id = self.client_manager.next_client_id();

        let mut client = Client::new(
            id,
            read_half,
            self.tx.clone(),
            self.client_manager.clone(),
            self.is_from_client,
        );

        let write_half = Arc::new(tokio::sync::Mutex::new(write_half));

        let mut clients = self.client_manager.clients.lock();
        let prev = clients.insert(id, write_half);
        drop(clients);
        assert!(prev.is_none());

        tokio::spawn(async move {
            client.start().await;
        });
        info!("client_manager started client {}", id);
    }

    pub async fn write(&self, client_id: i64, buf: String) {
        self.client_manager.write(client_id, buf).await
    }

    pub fn stop_all(&self) {
        self.client_manager.stop_all();
    }
}
