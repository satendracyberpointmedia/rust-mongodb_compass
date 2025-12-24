use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Credential {
    service: String,
    username: String,
    password: String,
}

pub fn save(service: &str, username: &str, password: &str) -> Result<()> {
    let credentials_path = get_credentials_path()?;
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = credentials_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create credentials directory")?;
    }
    
    // Load existing credentials
    let mut credentials = load_all().unwrap_or_default();
    
    // Remove existing credential for this service/username if it exists
    credentials.retain(|c| !(c.service == service && c.username == username));
    
    // Add new credential
    credentials.push(Credential {
        service: service.to_string(),
        username: username.to_string(),
        password: password.to_string(),
    });
    
    // Save to file
    let json = serde_json::to_string_pretty(&credentials)
        .context("Failed to serialize credentials")?;
    
    fs::write(&credentials_path, json)
        .context("Failed to write credentials file")?;
    
    Ok(())
}

pub fn load(service: &str, username: &str) -> Option<String> {
    let credentials = load_all().ok()?;
    credentials
        .into_iter()
        .find(|c| c.service == service && c.username == username)
        .map(|c| c.password)
}

pub fn load_all() -> Result<Vec<Credential>> {
    let credentials_path = get_credentials_path()?;
    
    if !credentials_path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&credentials_path)
        .context("Failed to read credentials file")?;
    
    let credentials: Vec<Credential> = serde_json::from_str(&content)
        .context("Failed to parse credentials file")?;
    
    Ok(credentials)
}

pub fn delete(service: &str, username: &str) -> Result<()> {
    let credentials_path = get_credentials_path()?;
    
    if !credentials_path.exists() {
        return Ok(());
    }
    
    let mut credentials = load_all()?;
    let initial_len = credentials.len();
    credentials.retain(|c| !(c.service == service && c.username == username));
    
    if credentials.len() < initial_len {
        let json = serde_json::to_string_pretty(&credentials)
            .context("Failed to serialize credentials")?;
        fs::write(&credentials_path, json)
            .context("Failed to write credentials file")?;
    }
    
    Ok(())
}

fn get_credentials_path() -> Result<PathBuf> {
    // Use platform-specific data directory
    let mut path = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;
    
    path.push("novadb-studio");
    path.push("credentials.json");
    
    Ok(path)
}
