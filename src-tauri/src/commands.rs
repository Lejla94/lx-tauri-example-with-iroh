use crate::{AppState};
use iroh::NodeAddr;
use iroh::NodeId;
use iroh::PublicKey;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use n0_future::StreamExt;

use tauri::State;
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageHistoryItem {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub last_seen: SystemTime,
    pub message_count: u64,
}

#[derive(serde::Serialize, Clone)]
struct ConnectionEvent {
    peer_id: String,
    status: String, // "connected" or "disconnected"
}



#[tauri::command]
pub async fn initialize_client(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<NodeInfo, String> {
    let node = crate::node::LXNode::spawn()
        .await
        .map_err(|e| e.to_string())?;
    let mut events = node.accept_events();
    let _ = app_handle.emit(
        "connection",
        ConnectionEvent {
            peer_id: node.endpoint().node_id().to_string(),
            status: "connected".to_string(),
        },
    );
    let node_info = NodeInfo {
        node_id:  node.endpoint().node_id().to_string(),
        public_key: "client.get_public_key()".to_string(),
    };
    // Background task for emitting events
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.next().await {
            // let _ = app_handle.emit(
            //     "message",
            //     MessageEvent {
            //         sender: peer_id.to_string(),
            //         content,
            //     },
            // );
            println!("tauri event LXLXL {:?}", event);
        }
    });

   Ok(node_info)
}


#[tauri::command]
pub async fn send_message(
    state: tauri::State<'_, AppState>,
    peer_id: String,
    content: String,
) -> Result<(), String> {
    let node = crate::node::LXNode::spawn()
        .await
        .map_err(|e| e.to_string())?;

    let node_id = peer_id
        .parse::<iroh::NodeId>()
        .map_err(|e| format!("Invalid peer_id: {}", e))?;

    let mut events = node.connect(node_id, content);

    // Spawn background task for stream processing
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.next().await {
            println!("event {event:?}");

            println!("tauri ASDF 2 event LXLXL {:?}", event);
            // You can emit this to frontend if needed
        }
    });

    Ok(())
}


