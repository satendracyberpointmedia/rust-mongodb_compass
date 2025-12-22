mod app;
mod mongo;
mod security;
mod utils;

use app::state::AppState;
use std::collections::HashMap;

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            client: std::sync::Mutex::new(None),
            cursors: std::sync::Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::connect_db,
            app::commands::list_databases,
            app::commands::start_find,
            app::commands::fetch_next,
            app::commands::cancel_query,
        ])
        .run(tauri::generate_context!())
        .expect("error running NovaDB Studio");
}
