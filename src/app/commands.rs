use tauri::State;
use uuid::Uuid;
use serde_json::Value;
use mongodb::bson::Document;

use crate::app::state::AppState;
use crate::mongo::{client, query, aggregation, index};
use crate::mongo::cursor_engine::CursorSession;
use crate::utils::json;

#[tauri::command]
pub async fn connect_db(uri: String, state: State<'_, AppState>) -> Result<(), String> {
    let client = client::connect(&uri).await.map_err(|e| e.to_string())?;
    *state.client.lock().map_err(|e| format!("Lock error: {}", e))? = Some(client);
    Ok(())
}

#[tauri::command]
pub async fn disconnect_db(state: State<'_, AppState>) -> Result<(), String> {
    *state.client.lock().map_err(|e| format!("Lock error: {}", e))? = None;
    state.cursors.lock().map_err(|e| format!("Lock error: {}", e))?.clear();
    Ok(())
}

#[tauri::command]
pub async fn list_databases(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let client = state.client.lock().map_err(|e| format!("Lock error: {}", e))?;
    let client = client.as_ref().ok_or("Not connected to database")?;
    client.list_database_names(None, None).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_collections(
    db: String,
    state: State<'_, AppState>
) -> Result<Vec<String>, String> {
    let client = state.client.lock().map_err(|e| format!("Lock error: {}", e))?;
    let client = client.as_ref().ok_or("Not connected to database")?;
    let database = client.database(&db);
    database.list_collection_names(None).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_find(
    db: String,
    collection: String,
    filter: Value,
    sort: Option<Value>,
    limit: Option<u64>,
    skip: Option<u64>,
    projection: Option<Value>,
    state: State<'_, AppState>
) -> Result<String, String> {
    let client = state.client.lock().map_err(|e| format!("Lock error: {}", e))?;
    let client = client.as_ref().ok_or("Not connected to database")?;

    let filter_doc: Document = json::json_to_bson(filter)?;
    let sort_doc = sort.map(|s| json::json_to_bson(s)).transpose()?;
    let projection_doc = projection.map(|p| json::json_to_bson(p)).transpose()?;

    let cursor = query::find_with_options(
        client.database(&db).collection(&collection),
        filter_doc,
        sort_doc,
        limit,
        skip,
        projection_doc,
    ).await.map_err(|e| e.to_string())?;

    let session_id = Uuid::new_v4().to_string();
    state.cursors.lock().map_err(|e| format!("Lock error: {}", e))?.insert(
        session_id.clone(),
        CursorSession { cursor, batch_size: 50 }
    );

    Ok(session_id)
}

#[tauri::command]
pub async fn start_aggregate(
    db: String,
    collection: String,
    pipeline: Vec<Value>,
    state: State<'_, AppState>
) -> Result<String, String> {
    let client = state.client.lock().map_err(|e| format!("Lock error: {}", e))?;
    let client = client.as_ref().ok_or("Not connected to database")?;

    let pipeline_docs: Result<Vec<Document>, String> = pipeline
        .into_iter()
        .map(|v| json::json_to_bson(v))
        .collect();

    let cursor = aggregation::aggregate(
        client.database(&db).collection(&collection),
        pipeline_docs?,
    ).await.map_err(|e| e.to_string())?;

    let session_id = Uuid::new_v4().to_string();
    state.cursors.lock().map_err(|e| format!("Lock error: {}", e))?.insert(
        session_id.clone(),
        CursorSession { cursor, batch_size: 50 }
    );

    Ok(session_id)
}

#[tauri::command]
pub async fn list_indexes(
    db: String,
    collection: String,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    let client = state.client.lock().map_err(|e| format!("Lock error: {}", e))?;
    let client = client.as_ref().ok_or("Not connected to database")?;

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
