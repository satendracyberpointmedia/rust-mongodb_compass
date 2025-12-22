use mongodb::{Collection, bson::Document};
use futures::StreamExt;

pub async fn list_indexes(
    collection: Collection<Document>
) -> mongodb::error::Result<Vec<Document>> {
    let mut cursor = collection.list_indexes(None).await?;
    let mut indexes = Vec::new();

    while let Some(index) = cursor.next().await {
        if let Ok(doc) = index {
            indexes.push(doc);
        }
    }

    Ok(indexes)
}
