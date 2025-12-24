mod app;
mod mongo;
mod security;
mod utils;

use app::state::AppState;
use std::collections::HashMap;

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            clients: std::sync::Mutex::new(HashMap::new()),
            connections: std::sync::Mutex::new(HashMap::new()),
            cursors: std::sync::Mutex::new(HashMap::new()),
            query_history: std::sync::Mutex::new(Vec::new()),
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
        ])
        .run(tauri::generate_context!())
        .expect("error running NovaDB Studio");
}
