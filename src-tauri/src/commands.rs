use crate::{AppState, P2PClient};
use iroh::NodeId;
use std::path::PathBuf;
use tauri::State;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tauri::AppHandle;
use tauri::Emitter;
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

#[tauri::command]
pub async fn initialize_client(
    app_handle: AppHandle,
    state: State<'_, AppState>
) -> Result<NodeInfo, String> {
    let mut client_guard = state.client.lock().await;
    
    if client_guard.is_none() {
        let mut client = P2PClient::new(state.data_dir.to_str().unwrap())
            .await
            .map_err(|e| format!("Failed to initialize P2P client: {}", e))?;
        
        let node_info = NodeInfo {
            node_id: client.get_node_id().to_string(),
            public_key: client.get_public_key(),
        };
        
        // Start the client in a background task
        let client_clone = client.clone();
        let app_handle_clone = app_handle.clone(); // Remove the & here
        tokio::spawn(async move {
            if let Err(e) = client_clone.start(app_handle_clone.clone()).await {
                eprintln!("P2P client error tauri testing lx command: {}", e);
                // Emit error to frontend
                let _ = app_handle_clone.emit("error", format!("P2P client error: {}", e));
            }
        });

        *client_guard = Some(client);
        Ok(node_info)
    } else {
        Err("Client already initialized".to_string())
    }
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    peer_id: String,
    content: String,
) -> Result<(), String> {
    let client_guard = state.client.lock().await;
    
    if let Some(client) = client_guard.as_ref() {
        let peer_id = peer_id.parse::<NodeId>()
            .map_err(|e| format!("Invalid peer ID: {}", e))?;
        
        client.send_message(peer_id, content)
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;
        
        Ok(())
    } else {
        Err("Client not initialized".to_string())
    }
}

// #[tauri::command]
// pub async fn get_message_history(
//     state: State<'_, AppState>,
//     peer_id: String,
//     limit: u64,
// ) -> Result<Vec<MessageHistoryItem>, String> {
//     let client_guard = state.client.lock().await;
//     
//     if let Some(client) = client_guard.as_ref() {
//         let peer_id = peer_id.parse::<NodeId>()
//             .map_err(|e| format!("Invalid peer ID: {}", e))?;
//         
//         let messages = client.get_message_history(peer_id, limit)
//             .await
//             .map_err(|e| format!("Failed to get message history: {}", e))?;
//         
//         let history: Vec<MessageHistoryItem> = messages
//             .into_iter()
//             .map(|msg| MessageHistoryItem {
//                 id: msg.id,
//                 sender: msg.sender.to_string(),
//                 recipient: msg.recipient.to_string(),
//                 content: msg.content,
//                 timestamp: msg.timestamp,
//             })
//             .collect();
//         
//         Ok(history)
//     } else {
//         Err("Client not initialized".to_string())
//     }
// }
//
// #[tauri::command]
// pub async fn get_peer_list(
//     state: State<'_, AppState>,
// ) -> Result<Vec<PeerInfo>, String> {
//     let client_guard = state.client.lock().await;
//     
//     if let Some(client) = client_guard.as_ref() {
//         let peers = client.get_peer_list()
//             .await
//             .map_err(|e| format!("Failed to get peer list: {}", e))?;
//         
//         Ok(peers.into_iter().map(|p| PeerInfo {
//             peer_id: p.node_id.to_string(),
//             last_seen: p.last_seen,
//             message_count: p.message_count,
//         }).collect())
//     } else {
//         Err("Client not initialized".to_string())
//     }
// }

#[tauri::command]
pub async fn get_node_info(
    state: State<'_, AppState>,
) -> Result<NodeInfo, String> {
    let client_guard = state.client.lock().await;
    
    if let Some(client) = client_guard.as_ref() {
        Ok(NodeInfo {
            node_id: client.get_node_id().to_string(),
            public_key: client.get_public_key(),
        })
    } else {
        Err("Client not initialized".to_string())
    }
}

// #[tauri::command]
// pub async fn cleanup_messages(
//     state: State<'_, AppState>,
// ) -> Result<(), String> {
//     let client_guard = state.client.lock().await;
//     
//     if let Some(client) = client_guard.as_ref() {
//         client.cleanup_old_messages()
//             .await
//             .map_err(|e| format!("Failed to cleanup messages: {}", e))?;
//         
//         Ok(())
//     } else {
//         Err("Client not initialized".to_string())
//     }
// }
