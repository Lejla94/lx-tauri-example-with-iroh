mod commands;
pub mod node;

// Re-export P2PClient for easier importing elsewhere
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;
pub struct AppState {
    client: Arc<Mutex<Option<crate::node::LXNode>>>,
    data_dir: PathBuf, // Add data_dir to AppState
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_pinia::init())
        .plugin(tauri_plugin_geolocation::init())
        .plugin(tauri_plugin_app_events::init())
        // .manage(app_state)
        .setup(|app| {
            // Get application data directory
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            // Create directory if it doesn't exist
            std::fs::create_dir_all(&data_dir).expect("Failed to create app data dir");

            // Initialize application state
            let app_state = AppState {
                client: Arc::new(Mutex::new(None)),
                data_dir, // Store data_dir in AppState
            };

            // Manage the complete state - this MUST be inside setup()
            app.manage(app_state);

            println!("App state initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::initialize_client,
            commands::send_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
