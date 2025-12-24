use mongodb::{Client, options::ClientOptions};
use anyhow::{Result, Context};

pub async fn connect(uri: &str) -> Result<Client> {
    let mut options = ClientOptions::parse(uri)
        .await
        .context("Failed to parse MongoDB connection URI")?;
    
    options.app_name = Some("NovaDB Studio".into());
    
    let client = Client::with_options(options)
        .context("Failed to create MongoDB client with options")?;
    
    // Test the connection
    client
        .database("admin")
        .run_command(mongodb::bson::doc! {"ping": 1}, None)
        .await
        .context("Failed to ping MongoDB server - connection test failed")?;
    
    Ok(client)
}
