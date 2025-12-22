use mongodb::{Client, options::ClientOptions};
use anyhow::Result;

pub async fn connect(uri: &str) -> Result<Client> {
    let mut options = ClientOptions::parse(uri).await?;
    options.app_name = Some("NovaDB Studio".into());
    Ok(Client::with_options(options)?)
}
