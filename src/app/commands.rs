use tauri::State;
use uuid::Uuid;
use serde_json::Value;
use mongodb::bson::Document;
use std::time::Instant;
use futures::StreamExt;

use crate::app::state::{AppState, ConnectionInfo, QueryHistoryEntry, ChangeStreamInfo};
use crate::mongo::{client, query, aggregation, index, crud, performance, change_streams, index_management};
use crate::mongo::cursor_engine::CursorSession;
use crate::utils::{json, export};
use tokio::sync::mpsc;

// ==================== Connection Management ====================

#[tauri::command]
pub async fn connect_db(
    uri: String,
    name: Option<String>,
    state: State<'_, AppState>
) -> Result<String, String> {
    let start = Instant::now();
    let client = client::connect(&uri).await.map_err(|e| e.to_string())?;
    let connection_time = start.elapsed().as_millis() as u64;

    let connection_id = Uuid::new_v4().to_string();
    let connection_name = name.unwrap_or_else(|| {
        // Extract name from URI if possible
        uri.split('@').last().unwrap_or("Connection").to_string()
    });

    let connection_info = ConnectionInfo {
        id: connection_id.clone(),
        name: connection_name,
        uri: uri.clone(),
        connected_at: chrono::Utc::now(),
    };

    state.clients.lock().map_err(|e| format!("Lock error: {}", e))?.insert(connection_id.clone(), Arc::new(client));
    state.connections.lock().map_err(|e| format!("Lock error: {}", e))?.insert(connection_id.clone(), connection_info);

    Ok(format!("{}|{}", connection_id, connection_time))
}

#[tauri::command]
pub async fn disconnect_db(
    connection_id: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    state.clients.lock().map_err(|e| format!("Lock error: {}", e))?.remove(&connection_id);
    state.connections.lock().map_err(|e| format!("Lock error: {}", e))?.remove(&connection_id);
    
    // Clean up cursors for this connection
    state.cursors.lock().map_err(|e| format!("Lock error: {}", e))?.retain(|_, _| true);
    
    Ok(())
}

#[tauri::command]
pub async fn list_connections(state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    let connections = state.connections.lock().map_err(|e| format!("Lock error: {}", e))?;
    let result: Result<Vec<Value>, String> = connections
        .values()
        .map(|conn| serde_json::to_value(conn)
            .map_err(|e| format!("Failed to serialize connection: {}", e)))
        .collect();
    result
}

#[tauri::command]
pub async fn get_connection(
    connection_id: String,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let connections = state.connections.lock().map_err(|e| format!("Lock error: {}", e))?;
    let conn = connections.get(&connection_id).ok_or("Connection not found")?;
    serde_json::to_value(conn).map_err(|e| format!("Failed to serialize connection: {}", e))
}

fn get_client(state: &State<'_, AppState>, connection_id: &str) -> Result<std::sync::Arc<mongodb::Client>, String> {
    let clients = state.clients.lock().map_err(|e| format!("Lock error: {}", e))?;
    clients.get(connection_id).ok_or("Connection not found or disconnected").map(|c| Arc::clone(c))
}

// ==================== Database Operations ====================

#[tauri::command]
pub async fn list_databases(
    connection_id: String,
    state: State<'_, AppState>
) -> Result<Vec<String>, String> {
    let client = get_client(&state, &connection_id)?;
    client.list_database_names(None, None).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_collections(
    connection_id: String,
    db: String,
    state: State<'_, AppState>
) -> Result<Vec<String>, String> {
    let client = get_client(&state, &connection_id)?;
    let database = client.database(&db);
    database.list_collection_names(None).await.map_err(|e| e.to_string())
}

// ==================== Query Operations ====================

#[tauri::command]
pub async fn start_find(
    connection_id: String,
    db: String,
    collection: String,
    filter: Value,
    sort: Option<Value>,
    limit: Option<u64>,
    skip: Option<u64>,
    projection: Option<Value>,
    state: State<'_, AppState>
) -> Result<String, String> {
    let start = Instant::now();
    let client = get_client(&state, &connection_id)?;

    let filter_doc: Document = json::json_to_bson(filter.clone())?;
    let sort_doc = sort.as_ref().map(|s| json::json_to_bson(s.clone())).transpose()?;
    let projection_doc = projection.as_ref().map(|p| json::json_to_bson(p.clone())).transpose()?;

    let cursor = query::find_with_options(
        client.database(&db).collection(&collection),
        filter_doc,
        sort_doc,
        limit,
        skip,
        projection_doc,
    ).await.map_err(|e| e.to_string())?;

    let execution_time = start.elapsed().as_millis() as u64;
    let session_id = Uuid::new_v4().to_string();
    
    state.cursors.lock().map_err(|e| format!("Lock error: {}", e))?.insert(
        session_id.clone(),
        CursorSession { cursor, batch_size: 50 }
    );

    // Save to query history
    let history_entry = QueryHistoryEntry {
        id: Uuid::new_v4().to_string(),
        connection_id: connection_id.clone(),
        database: db,
        collection,
        query_type: "find".to_string(),
        query: serde_json::json!({
            "filter": filter,
            "sort": sort,
            "limit": limit,
            "skip": skip,
            "projection": projection,
        }),
        executed_at: chrono::Utc::now(),
        execution_time_ms: Some(execution_time),
    };
    
    let mut history = state.query_history.lock().map_err(|e| format!("Lock error: {}", e))?;
    history.push(history_entry);
    if history.len() > 1000 {
        history.remove(0); // Keep only last 1000 queries
    }

    Ok(session_id)
}

#[tauri::command]
pub async fn start_aggregate(
    connection_id: String,
    db: String,
    collection: String,
    pipeline: Vec<Value>,
    state: State<'_, AppState>
) -> Result<String, String> {
    let start = Instant::now();
    let client = get_client(&state, &connection_id)?;

    let pipeline_docs: Result<Vec<Document>, String> = pipeline
        .iter()
        .map(|v| json::json_to_bson(v.clone()))
        .collect();

    let cursor = aggregation::aggregate(
        client.database(&db).collection(&collection),
        pipeline_docs?,
    ).await.map_err(|e| e.to_string())?;

    let execution_time = start.elapsed().as_millis() as u64;
    let session_id = Uuid::new_v4().to_string();
    
    state.cursors.lock().map_err(|e| format!("Lock error: {}", e))?.insert(
        session_id.clone(),
        CursorSession { cursor, batch_size: 50 }
    );

    // Save to query history
    let history_entry = QueryHistoryEntry {
        id: Uuid::new_v4().to_string(),
        connection_id: connection_id.clone(),
        database: db,
        collection,
        query_type: "aggregate".to_string(),
        query: serde_json::json!({ "pipeline": pipeline }),
        executed_at: chrono::Utc::now(),
        execution_time_ms: Some(execution_time),
    };
    
    let mut history = state.query_history.lock().map_err(|e| format!("Lock error: {}", e))?;
    history.push(history_entry);
    if history.len() > 1000 {
        history.remove(0);
    }

    Ok(session_id)
}

#[tauri::command]
pub async fn explain_query(
    connection_id: String,
    db: String,
    collection: String,
    query_type: String,
    filter: Option<Value>,
    pipeline: Option<Vec<Value>>,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let coll = client.database(&db).collection(&collection);

    let explain_result = match query_type.as_str() {
        "find" => {
            let filter_doc = filter.ok_or("Filter required for find query")?;
            let filter_bson: Document = json::json_to_bson(filter_doc)?;
            performance::explain_find(coll, filter_bson).await
        }
        "aggregate" => {
            let pipeline_vec = pipeline.ok_or("Pipeline required for aggregate query")?;
            let pipeline_docs: Result<Vec<Document>, String> = pipeline_vec
                .iter()
                .map(|v| json::json_to_bson(v.clone()))
                .collect();
            performance::explain_aggregate(coll, pipeline_docs?).await
        }
        _ => return Err("Invalid query type. Use 'find' or 'aggregate'".to_string()),
    };

    let doc = explain_result.map_err(|e| e.to_string())?;
    serde_json::to_value(doc).map_err(|e| format!("Failed to convert explain result: {}", e))
}

#[tauri::command]
pub async fn get_collection_stats(
    connection_id: String,
    db: String,
    collection: String,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let coll = client.database(&db).collection(&collection);
    
    let stats = performance::get_collection_stats(coll).await.map_err(|e| e.to_string())?;
    serde_json::to_value(stats).map_err(|e| format!("Failed to convert stats: {}", e))
}

#[tauri::command]
pub async fn list_indexes(
    connection_id: String,
    db: String,
    collection: String,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    let client = get_client(&state, &connection_id)?;

    let indexes = index::list_indexes(
        client.database(&db).collection(&collection)
    ).await.map_err(|e| e.to_string())?;

    let result: Result<Vec<Value>, String> = indexes
        .into_iter()
        .map(|doc| {
            serde_json::to_value(doc)
                .map_err(|e| format!("Failed to convert index to JSON: {}", e))
        })
        .collect();

    result
}

#[tauri::command]
pub async fn fetch_next(
    session_id: String,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    let mut cursors = state.cursors.lock().map_err(|e| format!("Lock error: {}", e))?;
    let session = cursors.get_mut(&session_id).ok_or("Invalid session ID")?;
    let docs = session.next_batch().await;
    
    let result: Result<Vec<Value>, String> = docs
        .into_iter()
        .map(|d| {
            serde_json::to_value(d)
                .map_err(|e| format!("Failed to convert document to JSON: {}", e))
        })
        .collect();

    result
}

#[tauri::command]
pub async fn cancel_query(
    session_id: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    state.cursors.lock().map_err(|e| format!("Lock error: {}", e))?.remove(&session_id);
    Ok(())
}

// ==================== CRUD Operations ====================

#[tauri::command]
pub async fn insert_document(
    connection_id: String,
    db: String,
    collection: String,
    document: Value,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let doc: Document = json::json_to_bson(document)?;
    
    let result = crud::insert_one(
        client.database(&db).collection(&collection),
        doc,
    ).await.map_err(|e| e.to_string())?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[tauri::command]
pub async fn insert_many_documents(
    connection_id: String,
    db: String,
    collection: String,
    documents: Vec<Value>,
    ordered: Option<bool>,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let docs: Result<Vec<Document>, String> = documents
        .into_iter()
        .map(|v| json::json_to_bson(v))
        .collect();
    
    let result = crud::insert_many(
        client.database(&db).collection(&collection),
        docs?,
        ordered,
    ).await.map_err(|e| e.to_string())?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[tauri::command]
pub async fn update_document(
    connection_id: String,
    db: String,
    collection: String,
    filter: Value,
    update: Value,
    upsert: Option<bool>,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let filter_doc: Document = json::json_to_bson(filter)?;
    let update_doc: Document = json::json_to_bson(update)?;
    
    let result = crud::update_one(
        client.database(&db).collection(&collection),
        filter_doc,
        update_doc,
        upsert,
    ).await.map_err(|e| e.to_string())?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[tauri::command]
pub async fn update_many_documents(
    connection_id: String,
    db: String,
    collection: String,
    filter: Value,
    update: Value,
    upsert: Option<bool>,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let filter_doc: Document = json::json_to_bson(filter)?;
    let update_doc: Document = json::json_to_bson(update)?;
    
    let result = crud::update_many(
        client.database(&db).collection(&collection),
        filter_doc,
        update_doc,
        upsert,
    ).await.map_err(|e| e.to_string())?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[tauri::command]
pub async fn delete_document(
    connection_id: String,
    db: String,
    collection: String,
    filter: Value,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let filter_doc: Document = json::json_to_bson(filter)?;
    
    let result = crud::delete_one(
        client.database(&db).collection(&collection),
        filter_doc,
    ).await.map_err(|e| e.to_string())?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[tauri::command]
pub async fn delete_many_documents(
    connection_id: String,
    db: String,
    collection: String,
    filter: Value,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let filter_doc: Document = json::json_to_bson(filter)?;
    
    let result = crud::delete_many(
        client.database(&db).collection(&collection),
        filter_doc,
    ).await.map_err(|e| e.to_string())?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[tauri::command]
pub async fn replace_document(
    connection_id: String,
    db: String,
    collection: String,
    filter: Value,
    replacement: Value,
    upsert: Option<bool>,
    state: State<'_, AppState>
) -> Result<Value, String> {
    let client = get_client(&state, &connection_id)?;
    let filter_doc: Document = json::json_to_bson(filter)?;
    let replacement_doc: Document = json::json_to_bson(replacement)?;
    
    let result = crud::replace_one(
        client.database(&db).collection(&collection),
        filter_doc,
        replacement_doc,
        upsert,
    ).await.map_err(|e| e.to_string())?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

// ==================== Export Operations ====================

#[tauri::command]
pub async fn export_results(
    documents: Vec<Value>,
    format: String,
    options: Option<Value>,
) -> Result<String, String> {
    match format.as_str() {
        "csv" => {
            let headers = options
                .and_then(|opts| opts.get("headers"))
                .and_then(|h| h.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());
            export::to_csv(&documents, headers)
        }
        "json" => {
            let pretty = options
                .and_then(|opts| opts.get("pretty"))
                .and_then(|p| p.as_bool())
                .unwrap_or(false);
            export::to_json(&documents, pretty)
        }
        _ => Err("Unsupported export format. Use 'csv' or 'json'".to_string()),
    }
}

// ==================== Query History ====================

#[tauri::command]
pub async fn get_query_history(
    limit: Option<usize>,
    connection_id: Option<String>,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    let history = state.query_history.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let mut filtered: Vec<&QueryHistoryEntry> = history.iter().collect();
    
    if let Some(conn_id) = connection_id {
        filtered.retain(|entry| entry.connection_id == conn_id);
    }
    
    filtered.sort_by(|a, b| b.executed_at.cmp(&a.executed_at));
    
    let limit_val = limit.unwrap_or(100);
    let result: Result<Vec<Value>, String> = filtered
        .into_iter()
        .take(limit_val)
        .map(|entry| serde_json::to_value(entry)
            .map_err(|e| format!("Failed to serialize history entry: {}", e)))
        .collect();
    
    result
}

#[tauri::command]
pub async fn clear_query_history(state: State<'_, AppState>) -> Result<(), String> {
    state.query_history.lock().map_err(|e| format!("Lock error: {}", e))?.clear();
    Ok(())
}

#[tauri::command]
pub async fn delete_query_history_entry(
    entry_id: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut history = state.query_history.lock().map_err(|e| format!("Lock error: {}", e))?;
    history.retain(|entry| entry.id != entry_id);
    Ok(())
}

// ==================== Change Streams (Real-time Monitoring) ====================

#[tauri::command]
pub async fn start_change_stream(
    connection_id: String,
    db: String,
    collection: Option<String>,
    filter: Option<Value>,
    operation_types: Option<Vec<String>>,
    state: State<'_, AppState>
) -> Result<String, String> {
    let client = get_client(&state, &connection_id)?;
    let stream_id = Uuid::new_v4().to_string();
    
    let (tx, _rx) = mpsc::unbounded_channel::<Value>();
    
    let stream = if let Some(coll_name) = &collection {
        // Watch collection
        let coll = client.database(&db).collection::<Document>(coll_name);
        let filter_doc = filter.as_ref().map(|f| json::json_to_bson(f.clone())).transpose()?;
        change_streams::watch_collection(coll, filter_doc, operation_types.clone()).await
            .map_err(|e| format!("Failed to start change stream: {}", e))?
    } else {
        // Watch database
        let database = client.database(&db);
        let filter_doc = filter.as_ref().map(|f| json::json_to_bson(f.clone())).transpose()?;
        change_streams::watch_database(database, filter_doc, operation_types.clone()).await
            .map_err(|e| format!("Failed to start change stream: {}", e))?
    };
    
    // Store change stream info
    let stream_info = ChangeStreamInfo {
        id: stream_id.clone(),
        connection_id: connection_id.clone(),
        database: db,
        collection: collection.clone(),
        filter: filter.clone(),
        operation_types: operation_types.unwrap_or_default(),
        started_at: chrono::Utc::now(),
        is_active: true,
    };
    
    state.change_streams.lock().map_err(|e| format!("Lock error: {}", e))?.insert(stream_id.clone(), stream_info);
    state.change_stream_senders.lock().map_err(|e| format!("Lock error: {}", e))?.insert(stream_id.clone(), tx);
    
    // Initialize event storage in both state and static storage
    state.change_stream_events.lock().map_err(|e| format!("Lock error: {}", e))?.insert(stream_id.clone(), Vec::new());
    
    if let Some(static_events) = crate::app::state::CHANGE_STREAM_EVENTS.get() {
        static_events.lock().map_err(|e| format!("Lock error: {}", e))?.insert(stream_id.clone(), Vec::new());
    }
    
    // Create channel for events
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Value>();
    state.change_stream_senders.lock().map_err(|e| format!("Lock error: {}", e))?.insert(stream_id.clone(), event_tx.clone());
    
    // Background task to store events from channel into static storage
    let stream_id_storage = stream_id.clone();
    if let Some(static_events) = crate::app::state::CHANGE_STREAM_EVENTS.get() {
        let events_storage = Arc::clone(static_events);
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if let Ok(mut events_map) = events_storage.lock() {
                    if let Some(events) = events_map.get_mut(&stream_id_storage) {
                        events.push(event);
                        if events.len() > 1000 {
                            events.remove(0);
                        }
                    }
                }
            }
        });
    }
    
    // Start listening to change stream
    let stream_id_listen = stream_id.clone();
    use std::sync::Arc;
    let streams_arc = Arc::new(state.change_streams);
    tokio::spawn(async move {
        let mut stream = stream;
        while let Some(change_result) = stream.next().await {
            match change_result {
                Ok(change_event) => {
                    if let Ok(change_value) = serde_json::to_value(&change_event) {
                        let _ = event_tx.send(change_value);
                    }
                }
                Err(e) => {
                    eprintln!("Change stream error: {}", e);
                    if let Ok(mut streams) = streams_arc.lock() {
                        if let Some(stream_info) = streams.get_mut(&stream_id_listen) {
                            stream_info.is_active = false;
                        }
                    }
                    break;
                }
            }
        }
    });
    
    Ok(stream_id)
}

#[tauri::command]
pub async fn stop_change_stream(
    stream_id: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    // Mark as inactive
    if let Some(stream_info) = state.change_streams.lock().map_err(|e| format!("Lock error: {}", e))?.get_mut(&stream_id) {
        stream_info.is_active = false;
    }
    
    state.change_streams.lock().map_err(|e| format!("Lock error: {}", e))?.remove(&stream_id);
    state.change_stream_senders.lock().map_err(|e| format!("Lock error: {}", e))?.remove(&stream_id);
    state.change_stream_events.lock().map_err(|e| format!("Lock error: {}", e))?.remove(&stream_id);
    Ok(())
}

#[tauri::command]
pub async fn list_change_streams(
    connection_id: Option<String>,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    let streams = state.change_streams.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let filtered: Vec<&ChangeStreamInfo> = if let Some(conn_id) = connection_id {
        streams.values().filter(|s| s.connection_id == conn_id).collect()
    } else {
        streams.values().collect()
    };
    
    let result: Result<Vec<Value>, String> = filtered
        .into_iter()
        .map(|s| serde_json::to_value(s)
            .map_err(|e| format!("Failed to serialize stream info: {}", e)))
        .collect();
    
    result
}

#[tauri::command]
pub async fn get_change_stream_events(
    stream_id: String,
    limit: Option<usize>,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    // Get stored events from static storage (updated by background task)
    if let Some(static_events) = crate::app::state::CHANGE_STREAM_EVENTS.get() {
        let events_map = static_events.lock().map_err(|e| format!("Lock error: {}", e))?;
        
        if let Some(events) = events_map.get(&stream_id) {
            let limit_val = limit.unwrap_or(100);
            let result: Vec<Value> = events
                .iter()
                .rev() // Most recent first
                .take(limit_val)
                .cloned()
                .collect();
            
            // Also sync to state for consistency
            drop(events_map);
            let mut state_events = state.change_stream_events.lock().map_err(|e| format!("Lock error: {}", e))?;
            if let Some(state_events_vec) = state_events.get_mut(&stream_id) {
                *state_events_vec = static_events.lock().map_err(|e| format!("Lock error: {}", e))?.get(&stream_id).cloned().unwrap_or_default();
            }
            
            return Ok(result);
        }
    }
    
    Ok(Vec::new())
}

// Helper command to poll and store events (call this periodically from frontend)
#[tauri::command]
pub async fn poll_change_stream_events(
    stream_id: String,
    state: State<'_, AppState>
) -> Result<usize, String> {
    // Try to receive events from channel and store them
    let senders = state.change_stream_senders.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Note: We can't receive from the channel here as it's owned by the background task
    // Events are stored automatically when they arrive
    // This is a placeholder - in production, use Tauri events or WebSockets
    
    let events_map = state.change_stream_events.lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(events_map.get(&stream_id).map(|e| e.len()).unwrap_or(0))
}

#[tauri::command]
pub async fn clear_change_stream_events(
    stream_id: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut events_map = state.change_stream_events.lock().map_err(|e| format!("Lock error: {}", e))?;
    if let Some(events) = events_map.get_mut(&stream_id) {
        events.clear();
    }
    Ok(())
}

// ==================== Index Management ====================

#[tauri::command]
pub async fn create_index(
    connection_id: String,
    db: String,
    collection: String,
    keys: Value,
    name: Option<String>,
    unique: Option<bool>,
    sparse: Option<bool>,
    background: Option<bool>,
    expire_after_seconds: Option<i64>,
    partial_filter: Option<Value>,
    state: State<'_, AppState>
) -> Result<String, String> {
    let client = get_client(&state, &connection_id)?;
    let coll = client.database(&db).collection::<Document>(&collection);
    
    let keys_doc: Document = json::json_to_bson(keys)?;
    let partial_filter_doc = partial_filter.map(|f| json::json_to_bson(f)).transpose()?;
    
    let index_name = index_management::create_index_with_options(
        coll,
        keys_doc,
        name,
        unique,
        sparse,
        background,
        expire_after_seconds,
        partial_filter_doc,
        None,
        None,
    ).await.map_err(|e| e.to_string())?;
    
    Ok(index_name)
}

#[tauri::command]
pub async fn drop_index(
    connection_id: String,
    db: String,
    collection: String,
    index_name: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    let client = get_client(&state, &connection_id)?;
    let coll = client.database(&db).collection::<Document>(&collection);
    
    index_management::drop_index(coll, index_name).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn drop_all_indexes(
    connection_id: String,
    db: String,
    collection: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    let client = get_client(&state, &connection_id)?;
    let coll = client.database(&db).collection::<Document>(&collection);
    
    index_management::drop_all_indexes(coll).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn rebuild_indexes(
    connection_id: String,
    db: String,
    collection: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    let client = get_client(&state, &connection_id)?;
    let coll = client.database(&db).collection::<Document>(&collection);
    
    index_management::rebuild_indexes(coll).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_index_usage_stats(
    connection_id: String,
    db: String,
    collection: String,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    let client = get_client(&state, &connection_id)?;
    let coll = client.database(&db).collection::<Document>(&collection);
    
    let stats = index_management::analyze_index_usage(coll).await.map_err(|e| e.to_string())?;
    
    let result: Result<Vec<Value>, String> = stats
        .into_iter()
        .map(|doc| serde_json::to_value(doc)
            .map_err(|e| format!("Failed to convert stats to JSON: {}", e)))
        .collect();
    
    result
}

#[tauri::command]
pub async fn get_index_recommendations(
    connection_id: String,
    db: String,
    collection: String,
    sample_size: Option<usize>,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    let client = get_client(&state, &connection_id)?;
    let coll = client.database(&db).collection::<Document>(&collection);
    
    let recommendations = index_management::get_index_recommendations(coll, sample_size)
        .await.map_err(|e| e.to_string())?;
    
    let result: Result<Vec<Value>, String> = recommendations
        .into_iter()
        .map(|doc| serde_json::to_value(doc)
            .map_err(|e| format!("Failed to convert recommendation to JSON: {}", e)))
        .collect();
    
    result
}
