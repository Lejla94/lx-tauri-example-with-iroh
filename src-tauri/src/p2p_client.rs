use iroh::{endpoint::ConnectionError, RelayMode, SecretKey, NodeId};
use iroh::endpoint::{Endpoint, Connection, RecvStream, SendStream};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use std::time::SystemTime;
use sled;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use iroh::RelayMap;
use iroh::RelayUrl;
use url::Url;
use tauri::Emitter;
use tauri::AppHandle;
const MAX_STREAM_SIZE: usize = 1024 * 1024; // 1MB
const RELAY_SERVER: &str = "http://0.0.0.0:3340";


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConnection {
    pub node_id: NodeId,
    pub address: String,
    pub last_seen: SystemTime,
    pub message_count: u64,
}

#[derive(Clone)]
pub struct P2PClient {
    endpoint: Endpoint,
    peers_tree: sled::Db,
}

#[derive(serde::Serialize, Clone)]
struct MessageEvent {
    sender: String,
    content: String,:
}

#[derive(serde::Serialize, Clone)]
struct ConnectionEvent {
    peer_id: String,
    status: String, // "connected" or "disconnected"
}


impl P2PClient {
    pub async fn new(data_dir: &str) -> Result<Self> {
        let mut rng = rand::rngs::OsRng;
        let secret_key = SecretKey::generate(&mut rng);

        let endpoint = Endpoint::builder()
            // .discovery_n0()
            .relay_mode(RelayMode::Custom(RelayMap::from(
                RelayUrl::from(Url::parse(RELAY_SERVER)?),
            )))
            .secret_key(secret_key)
            .bind()
            .await?;

        // let mut builder = iroh::protocol::Router::builder(endpoint);
        // let router = builder.spawn();

        let peers_tree = sled::open(format!("{}/peers", data_dir))?;
        Ok(Self { endpoint, peers_tree })
    }

    pub fn get_node_id(&self) -> NodeId {
        self.endpoint.node_id()
    }

    pub fn get_public_key(&self) -> String {
        hex::encode(self.endpoint.secret_key().public().as_bytes())
    }

    pub async fn start(&self, app_handle: AppHandle) -> Result<()> {
        println!("🌐 Started node tauri: {}", self.get_node_id());
        println!("🔗 Relay tauri: {}", RELAY_SERVER);

        while let Some(connecting) = self.endpoint.accept().await {
            let this = self.clone();
            let app_handle = app_handle.clone();
            tokio::spawn(async move {
                if let Ok(conn) = connecting.await {
                    println!("tauri tauri tauri Connecting workds");
                   if let Err(e) = this.handle_connection(conn, app_handle).await {
                        eprintln!("Connection error: {:?}", e);
                    }
                }
            });
        }
        // for connection in incoming_connections {
        //     let this = self.clone();
        //     let app_handle = app_handle.clone();
        //     tokio::spawn(async move {
        //         if let Err(e) = this.handle_connection(connection, app_handle).await {
        //             eprintln!("Connection error: {:?}", e);
        //         }
        //     });
        // }
        Ok(())
    }

    async fn handle_connection(&self, connection: Connection, app_handle: AppHandle) -> Result<()> {
        let peer_id = connection.remote_node_id().context("Missing peer ID")?;
        println!("🔌tauri  Connection from: {:?}", peer_id);
        
        // Emit connection event
        let _ = app_handle.emit("connection", ConnectionEvent {
            peer_id: peer_id.to_string(),
            status: "connected".to_string(),
        });
        
        self.update_peer(peer_id).await?;

        loop {
            tokio::select! {
                Ok(stream) = connection.accept_uni() => {
                    let this = self.clone();
                    let peer_id = peer_id;
                    let app_handle = app_handle.clone();
                    println!("tauri  accept uni {:?}: ", peer_id );
                    tokio::spawn(async move {
                        if let Err(e) = this.handle_uni_stream(stream, peer_id, app_handle).await {
                            eprintln!("Uni stream error: {:?}", e);
                        }
                    });
                },
                Ok((send, recv)) = connection.accept_bi() => {
                    let this = self.clone();
                    let peer_id = peer_id;
                    let app_handle = app_handle.clone();
                    println!("tauri  accept bi {:?}: ", peer_id );
                    tokio::spawn(async move {
                        if let Err(e) = this.handle_bi_stream(send, recv, peer_id, app_handle).await {
                            eprintln!("Bi stream error: {:?}", e);
                        }
                    });
                },
                Ok(data) = connection.read_datagram() => {
                    println!("📨 tauri Datagram from {:?}: {} bytes", peer_id, data.len());
                    
                    // Handle datagram as message
                    if let Ok(content) = String::from_utf8(data.to_vec()) {
                        let _ = app_handle.emit("message", MessageEvent {
                            sender: peer_id.to_string(),
                            content,
                        });
                    }
                    self.update_peer_message_count(peer_id, 1).await?;
                },
                else => break,
            }
        }

        println!("🚫 Connection closed: {:?}", peer_id);
        // Emit disconnection event
        let _ = app_handle.emit("connection", ConnectionEvent {
            peer_id: peer_id.to_string(),
            status: "disconnected".to_string(),
        });
        Ok(())
    }

    async fn handle_uni_stream(
        &self,
        mut stream: RecvStream,
        peer_id: NodeId,
        app_handle: AppHandle
    ) -> Result<()> {
        println!("💬 Uni tauri: handle_uni_stream");
        let mut buf = vec![0; MAX_STREAM_SIZE];
        let n = stream.read(&mut buf).await?;
        buf.truncate(n.expect("MUST HAVE MESSAGE LX"));
        let content = String::from_utf8_lossy(&buf).to_string();
        
        println!("💬 Uni tauri: {}", content);
        
        // Emit message event
        let _ = app_handle.emit("message", MessageEvent {
            sender: peer_id.to_string(),
            content,
        });
        
        self.update_peer_message_count(peer_id, 1).await?;
        Ok(())
    }

    async fn handle_bi_stream(
        &self,
        mut send: SendStream,
        mut recv: RecvStream,
        peer_id: NodeId,
        app_handle: AppHandle
    ) -> Result<()> {
        let mut buf = vec![0; MAX_STREAM_SIZE];
        let n = recv.read(&mut buf).await?.min(Some(MAX_STREAM_SIZE));
        buf.truncate(n.expect("BI STREAM MESSAGE MUST EXIST LX"));
        let content = String::from_utf8_lossy(&buf).to_string();
        
        println!("💬 Bi: {}", content);
        
        // Emit message event
        let _ = app_handle.emit("message", MessageEvent {
            sender: peer_id.to_string(),
            content,
        });
        
        self.update_peer_message_count(peer_id, 1).await?;

        let response = format!("ACK from {}!", self.get_node_id());
        send.write_all(response.as_bytes()).await?;
        send.finish()?;
        Ok(())
    }    async fn update_peer(&self, node_id: NodeId) -> Result<()> {
        let key = format!("peer:{}", node_id);
        let now = SystemTime::now();

        let peer = match self.peers_tree.get(&key)? {
            Some(val) => {
                let mut peer: PeerConnection = serde_json::from_slice(&val)?;
                peer.last_seen = now;
                peer
            }
            None => PeerConnection {
                node_id,
                address: "<relay>".to_string(),
                last_seen: now,
                message_count: 0,
            },
        };

        self.peers_tree.insert(key, serde_json::to_vec(&peer)?)?;
        Ok(())
    }

    async fn update_peer_message_count(&self, node_id: NodeId, count: u64) -> Result<()> {
        let key = format!("peer:{}", node_id);
        if let Some(val) = self.peers_tree.get(&key)? {
            let mut peer: PeerConnection = serde_json::from_slice(&val)?;
            peer.message_count += count;
            peer.last_seen = SystemTime::now();
            self.peers_tree.insert(key, serde_json::to_vec(&peer)?)?;
        }
        Ok(())
    }

    pub async fn close(self) -> Result<()> {
        self.endpoint.close().await;
        Ok(())
    }

    /// Send a UTF-8 message to a peer using a unidirectional stream.
    pub async fn send_message(&self, peer_id: NodeId, content: String) -> Result<()> {
        println!("📤 Sending message to peer: {}", peer_id);

        let conn = self
            .endpoint
            .connect(peer_id, &[]) // ✅ FIXED: supply empty bytes
            .await
            .context("Failed to connect to peer")?;

        let mut stream = conn
            .open_uni()
            .await
            .context("Failed to open uni stream")?;

        stream
            .write_all(content.as_bytes())
            .await
            .context("Failed to write message")?;

        stream.finish().context("Failed to finish stream")?;

        println!("✅ Message sent successfully");
        Ok(())
    }
}

