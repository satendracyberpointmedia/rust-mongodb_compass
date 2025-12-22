use mongodb::{Collection, bson::Document};

pub async fn aggregate(
    collection: Collection<Document>,
    pipeline: Vec<Document>,
) -> mongodb::error::Result<mongodb::Cursor<Document>> {
    collection.aggregate(pipeline, None).await
}
