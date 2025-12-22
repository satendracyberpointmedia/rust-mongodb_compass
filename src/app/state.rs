use mongodb::Client;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::mongo::cursor_engine::CursorSession;

pub struct AppState {
    pub client: Mutex<Option<Client>>,
    pub cursors: Mutex<HashMap<String, CursorSession>>,
}
