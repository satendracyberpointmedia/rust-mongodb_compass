mod app;
mod mongo;
mod security;
mod utils;

use app::state::AppState;
use std::collections::HashMap;

fn main() {
    // Initialize static event storage
    app::state::CHANGE_STREAM_EVENTS.set(Arc::new(Mutex::new(HashMap::new())))
        .expect("Failed to initialize change stream events storage");
    
    tauri::Builder::default()
        .manage(AppState {
            clients: std::sync::Mutex::new(HashMap::new()),
            connections: std::sync::Mutex::new(HashMap::new()),
            cursors: std::sync::Mutex::new(HashMap::new()),
            query_history: std::sync::Mutex::new(Vec::new()),
            change_streams: std::sync::Mutex::new(HashMap::new()),
            change_stream_senders: std::sync::Mutex::new(HashMap::new()),
            change_stream_events: std::sync::Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            // Connection Management
            app::commands::connect_db,
            app::commands::disconnect_db,
            app::commands::list_connections,
            app::commands::get_connection,
            // Database Operations
            app::commands::list_databases,
            app::commands::list_collections,
            // Query Operations
            app::commands::start_find,
            app::commands::start_aggregate,
            app::commands::explain_query,
            app::commands::get_collection_stats,
            app::commands::list_indexes,
            app::commands::fetch_next,
            app::commands::cancel_query,
            // CRUD Operations
            app::commands::insert_document,
            app::commands::insert_many_documents,
            app::commands::update_document,
            app::commands::update_many_documents,
            app::commands::delete_document,
            app::commands::delete_many_documents,
            app::commands::replace_document,
            // Export Operations
            app::commands::export_results,
            // Query History
            app::commands::get_query_history,
            app::commands::clear_query_history,
            app::commands::delete_query_history_entry,
            // Change Streams (Real-time Monitoring)
            app::commands::start_change_stream,
            app::commands::stop_change_stream,
            app::commands::list_change_streams,
            app::commands::get_change_stream_events,
            app::commands::clear_change_stream_events,
            app::commands::poll_change_stream_events,
            // Index Management
            app::commands::create_index,
            app::commands::drop_index,
            app::commands::drop_all_indexes,
            app::commands::rebuild_indexes,
            app::commands::get_index_usage_stats,
            app::commands::get_index_recommendations,
        ])
        .run(tauri::generate_context!())
        .expect("error running NovaDB Studio");
}
