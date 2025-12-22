use tauri::api::passwords::{set_password, get_password};

pub fn save(service: &str, user: &str, password: &str) {
    let _ = set_password(service, user, password);
}

pub fn load(service: &str, user: &str) -> Option<String> {
    get_password(service, user).ok()
}
