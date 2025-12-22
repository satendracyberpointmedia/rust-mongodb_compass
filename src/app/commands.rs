use tauri::State;
use uuid::Uuid;
use serde_json::Value;
use mongodb::bson::Document;

use crate::app::state::AppState;
use crate::mongo::{client, query, aggregation, index};
use crate::mongo::cursor_engine::CursorSession;

#[tauri::command]
pub async fn connect_db(uri: String, state: State<'_, AppState>) -> Result<(), String> {
    let client = client::connect(&uri).await.map_err(|e| e.to_string())?;
    *state.client.lock().unwrap() = Some(client);
    Ok(())
}

#[tauri::command]
pub async fn list_databases(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let client = state.client.lock().unwrap();
    let client = client.as_ref().ok_or("Not connected")?;
    client.list_database_names(None, None).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_find(
    db: String,
    collection: String,
    filter: Value,
    state: State<'_, AppState>
) -> Result<String, String> {
    let client = state.client.lock().unwrap();
    let client = client.as_ref().ok_or("Not connected")?;

    let filter: Document = mongodb::bson::from_json(filter.to_string().as_str())
        .map_err(|e| e.to_string())?;

    let cursor = query::find(
        client.database(&db).collection(&collection),
        filter
    ).await.map_err(|e| e.to_string())?;

    let session_id = Uuid::new_v4().to_string();
    state.cursors.lock().unwrap().insert(
        session_id.clone(),
        CursorSession { cursor, batch_size: 50 }
    );

    Ok(session_id)
}

#[tauri::command]
pub async fn fetch_next(
    session_id: String,
    state: State<'_, AppState>
) -> Result<Vec<Value>, String> {
    let mut cursors = state.cursors.lock().unwrap();
    let session = cursors.get_mut(&session_id).ok_or("Invalid session")?;
    let docs = session.next_batch().await;
    Ok(docs.into_iter().map(|d| serde_json::to_value(d).unwrap()).collect())
}

#[tauri::command]
pub async fn cancel_query(
    session_id: String,
    state: State<'_, AppState>
) {
    state.cursors.lock().unwrap().remove(&session_id);
}
