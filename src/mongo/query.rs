use mongodb::{bson::Document, Collection};

pub async fn find(
    collection: Collection<Document>,
    filter: Document,
) -> mongodb::error::Result<mongodb::Cursor<Document>> {
    collection.find(filter, None).await
}
