use mongodb::Client;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use serde::{Serialize, Deserialize};

use crate::mongo::cursor_engine::CursorSession;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

pub struct AppState {
    pub clients: Mutex<HashMap<String, Arc<Client>>>,
    pub connections: Mutex<HashMap<String, ConnectionInfo>>,
    pub cursors: Mutex<HashMap<String, CursorSession>>,
    pub query_history: Mutex<Vec<QueryHistoryEntry>>,
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
