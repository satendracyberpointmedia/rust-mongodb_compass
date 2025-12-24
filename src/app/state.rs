use mongodb::Client;
use std::collections::HashMap;
use std::sync::{Mutex, Arc, OnceLock};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

use crate::mongo::cursor_engine::CursorSession;

// Static storage for change stream events (accessible from background tasks)
pub static CHANGE_STREAM_EVENTS: OnceLock<Arc<Mutex<HashMap<String, Vec<serde_json::Value>>>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeStreamInfo {
    pub id: String,
    pub connection_id: String,
    pub database: String,
    pub collection: Option<String>,
    pub filter: Option<serde_json::Value>,
    pub operation_types: Vec<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

pub struct AppState {
    pub clients: Mutex<HashMap<String, Arc<Client>>>,
    pub connections: Mutex<HashMap<String, ConnectionInfo>>,
    pub cursors: Mutex<HashMap<String, CursorSession>>,
    pub query_history: Mutex<Vec<QueryHistoryEntry>>,
    pub change_streams: Mutex<HashMap<String, ChangeStreamInfo>>,
    pub change_stream_senders: Mutex<HashMap<String, mpsc::UnboundedSender<serde_json::Value>>>,
    pub change_stream_events: Mutex<HashMap<String, Vec<serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: String,
    pub connection_id: String,
    pub database: String,
    pub collection: String,
    pub query_type: String, // "find", "aggregate", etc.
    pub query: serde_json::Value,
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub execution_time_ms: Option<u64>,
}
