mod client_manager;
mod kvstore;
mod peer;

use std::fs::read_to_string;
use crate::replicant::client_manager::{ClientManager, Message};
use crate::replicant::kvstore::memkvstore::MemKVStore;
use ::log::info;
use omnipaxos::*;
use omnipaxos_storage::memory_storage::MemoryStorage;
use serde_json::Value as json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use omnipaxos::util::{LogEntry, NodeId, SnapshottedEntry};
use tokio::sync::{mpsc, Mutex};
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time;
use crate::replicant::kvstore::{create_store, KVStore};
use crate::replicant::kvstore::kv::Command;
use crate::replicant::peer::TcpLink;
use crate::replicant::peer::Peer;

type OmniPaxosKV = OmniPaxos<Command, MemoryStorage<Command>>;

async fn make_peer(port: &json) -> Arc<TcpLink> {
    Arc::new(TcpLink::new(port.as_str().unwrap()).await)
}

struct ReplicantInner {
    id: i64,
    ip_port: String,
    paxos: Arc<RwLock<OmniPaxosKV>>,
    client_manager: ClientManager,
    peer_manager: ClientManager,
    peer_listener: TcpListener,
    peers: Vec<Peer>,
    last_decided_idx: Arc<Mutex<u64>>,
    store: Mutex<Box<dyn KVStore + Sync + Send>>,
    snapshot_interval: u64,
    recv: Arc<RwLock<UnboundedReceiver<Message>>>,
}

impl ReplicantInner {
    async fn new(config: &json) -> Self {
        let id = config["id"].as_i64().unwrap();
        let peers = config["peers"].as_array().unwrap();
        let ip_port = peers[id as usize].as_str().unwrap().to_string();
        let peer_listener = TcpListener::bind(ip_port.clone()).await.unwrap();

        let server_config = ServerConfig {
            pid: (id + 1) as NodeId,
            election_tick_timeout: 15,
            ..Default::default()
        };
        let cluster_config = ClusterConfig {
            configuration_id: 1,
            nodes: (1..=peers.len() as u64).collect(),
            ..Default::default()
        };
        let op_config = OmniPaxosConfig {
            server_config,
            cluster_config,
        };
        let paxos = Arc::new(RwLock::new(op_config
            .build(MemoryStorage::default())
            .expect("failed to build OmniPaxos")));

        let (sender, recv) = mpsc::unbounded_channel();

        let num_peers = peers.len() as i64;
        let client_manager =
            ClientManager::new(id, num_peers, sender.clone(), true);
        let peer_manager =
            ClientManager::new(id, num_peers, sender.clone(), false);
        let snapshot_interval = config["snapshot_interval"].as_u64().unwrap();

        let mut peers = Vec::new();
        for (id, port) in config["peers"].as_array().unwrap().iter().enumerate()
        {
            peers.push(Peer {
                id: id as i64,
                stub: make_peer(port).await,
            });
        }

        Self {
            id,
            ip_port,
            paxos,
            client_manager,
            peer_manager,
            peer_listener,
            peers,
            last_decided_idx: Arc::new(Mutex::new(0)),
            store: Mutex::new(create_store(config)),
            snapshot_interval,
            recv: Arc::new(RwLock::new(recv)),
        }
    }

    async fn executor_task_fn(&self) {
        let mut msg_interval = time::interval(Duration::from_millis(1));
        let mut tick_interval = time::interval(Duration::from_millis(10));
        loop {
            tokio::select! {
                biased;
                _ = msg_interval.tick() => {
                    self.process_incoming_msgs().await;
                    self.send_outgoing_msgs().await;
                    self.handle_decided_entries().await;
                },
                _ = tick_interval.tick() => {
                    let mut paxos = self.paxos.write().await;
                    paxos.tick();
                },
                else => (),
            }
        }
    }

    async fn server_task_fn(&self, mut shutdown: Receiver<()>) {
        let mut addr: SocketAddr = self.ip_port.parse().unwrap();
        addr.set_port(addr.port() + 1);
        let listener = TcpListener::bind(addr).await.unwrap();

        loop {
            tokio::select! {
                Ok((client, _)) = listener.accept() => {
                    self.client_manager.start(client)
                },
                _ = &mut shutdown => break,
            }
        }
    }

    async fn peer_server_task_fn(&self, mut shutdown: Receiver<()>) {
        loop {
            tokio::select! {
                Ok((client, _)) = self.peer_listener.accept() => {
                    self.peer_manager.start(client)
                },
                _ = &mut shutdown => break,
            }
        }
    }

    async fn process_incoming_msgs(&self) {
        let mut paxos = self.paxos.write().await;
        let waker = futures::task::noop_waker();
        let mut rx = self.recv.write().await;
        let mut cx = std::task::Context::from_waker(&waker);
        loop {
            match rx.poll_recv(&mut cx) {
                Poll::Ready(Some(msg)) => {
                    match msg {
                        Message::ClientRequest(cmd) => {
                            let result = paxos.append(cmd);
                            if result.is_err() {
                                match result.unwrap_err() {
                                    ProposeErr::PendingReconfigEntry(e) => info!("PendingReconfigEntry"),
                                    ProposeErr::PendingReconfigConfig(_, ..) => info!("PendingReconfigConfig"),
                                    ProposeErr::ConfigError(_, ..) => info!("ConfigError"),
                                }
                            }
                        }
                        Message::OmniPaxosMsg(msg) => {
                            paxos.handle_incoming(msg);
                        },
                        _ => unimplemented!(),
                    }
                }
                _ => { break; }
            }
        }
    }

    async fn send_outgoing_msgs(&self) {
        let mut paxos = self.paxos.write().await;
        let messages = paxos.outgoing_messages();
        for msg in messages {
            let receiver = msg.get_receiver() as usize - 1;
            if receiver == self.id as usize {
                continue;
            }
            let peer = self.peers.get(receiver).unwrap();
            peer.stub.send_await_response(msg);
        }
    }

    async fn handle_decided_entries(&self) {
        let mut paxos = self.paxos.write().await;
        let mut last_decided_idx = self.last_decided_idx.lock().await;
        let new_decided_idx = paxos.get_decided_idx();
        if *last_decided_idx < new_decided_idx as u64 {
            let decided_entries = paxos.read_decided_suffix(*last_decided_idx).unwrap();
            // info!("{}, decided entries len: {}", new_decided_idx, decided_entries.len());
            *last_decided_idx = new_decided_idx as u64;
            self.update_database(decided_entries).await;
        }
    }

    async fn update_database(&self, decided_entries: Vec<LogEntry<Command>>) {
        let mut store = self.store.lock().await;
        for entry in decided_entries {
            match entry {
                LogEntry::Decided(cmd) => {
                    let (result, client_id) = store.execute(cmd);
                    self.client_manager.write(client_id, result).await;
                }
                LogEntry::Undecided(_) => {info!("undecided entry")}
                LogEntry::Snapshotted(s) => {
                    info!("snapshotted entry, trim index: {}", s.trimmed_idx)}
                LogEntry::Trimmed(_) => {info!("Trimmed entry")}
                LogEntry::StopSign(_, ..) => {info!("stopsign entry")}
            }
        }
    }
}

pub struct Replicant {
    replicant: Arc<ReplicantInner>,
}

pub struct ReplicantHandle {
    executor_task_handle: JoinHandle<()>,
    server_task_handle: (JoinHandle<()>, Sender<()>),
    peer_server_task_handle: (JoinHandle<()>, Sender<()>),
}

impl Replicant {
    pub async fn new(config: &json) -> Self {
        Self {
            replicant: Arc::new(ReplicantInner::new(config).await),
        }
    }

    pub fn start(&self) -> ReplicantHandle {
        ReplicantHandle {
            executor_task_handle: self.start_executor_task(),
            server_task_handle: self.start_server_task(),
            peer_server_task_handle: self.start_peer_server_task(),
        }
    }

    pub async fn stop(&self, handle: ReplicantHandle) {
        self.stop_server_task(handle.server_task_handle).await;
        // self.stop_executor_task(handle.executor_task_handle).await;
        self.stop_peer_server_task(handle.peer_server_task_handle).await;
    }

    fn start_server_task(&self) -> (JoinHandle<()>, Sender<()>) {
        info!("{} starting server task", self.replicant.id);
        let (shutdown_send, shutdown_recv) = oneshot::channel();
        let replicant = self.replicant.clone();
        let handle = tokio::spawn(async move {
            replicant.server_task_fn(shutdown_recv).await;
        });
        (handle, shutdown_send)
    }

    async fn stop_server_task(&self, handle: (JoinHandle<()>, Sender<()>)) {
        info!("{} stopping server task", self.replicant.id);
        let (handle, shutdown) = handle;
        shutdown.send(()).unwrap();
        handle.await.unwrap();
        self.replicant.client_manager.stop_all();
    }

    fn start_peer_server_task(&self) -> (JoinHandle<()>, Sender<()>) {
        info!("{} starting paxos server for peer", self.replicant.id);
        let (shutdown_send, shutdown_recv) = oneshot::channel();
        let replicant = self.replicant.clone();
        let handle = tokio::spawn(async move {
            replicant.peer_server_task_fn(shutdown_recv).await;
        });
        (handle, shutdown_send)
    }

    async fn stop_peer_server_task(&self, handle: (JoinHandle<()>, Sender<()>)) {
        info!("{} stopping paxos server task", self.replicant.id);
        let (handle, shutdown) = handle;
        shutdown.send(()).unwrap();
        handle.await.unwrap();
        self.replicant.peer_manager.stop_all();
    }

    fn start_executor_task(&self) -> JoinHandle<()> {
        info!("{} starting executor task", self.replicant.id);
        let replicant = self.replicant.clone();
        tokio::spawn(async move {
            replicant.executor_task_fn().await;
        })
    }

    async fn stop_executor_task(&self, handle: JoinHandle<()>) {
        info!("{} stopping executor task", self.replicant.id);
        handle.await.unwrap();
    }
}
