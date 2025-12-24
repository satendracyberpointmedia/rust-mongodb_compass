use mongodb::{Collection, bson::Document};

pub async fn explain_find(
    collection: Collection<Document>,
    filter: Document,
) -> mongodb::error::Result<Document> {
    let db = collection.database();
    let coll_name = collection.name();
    
    // Use explain command directly
    db.run_command(
        mongodb::bson::doc! {
            "explain": mongodb::bson::doc! {
                "find": coll_name,
                "filter": filter
            },
            "verbosity": "executionStats"
        },
        None,
    ).await
}

pub async fn explain_aggregate(
    collection: Collection<Document>,
    pipeline: Vec<Document>,
) -> mongodb::error::Result<Document> {
    let db = collection.database();
    let coll_name = collection.name();
    
    // Use explain command directly
    db.run_command(
        mongodb::bson::doc! {
            "explain": mongodb::bson::doc! {
                "aggregate": coll_name,
                "pipeline": pipeline,
                "cursor": mongodb::bson::doc! {}
            },
            "verbosity": "executionStats"
        },
        None,
    ).await
}

pub async fn get_collection_stats(
    collection: Collection<Document>,
) -> mongodb::error::Result<Document> {
    let db = collection.database();
    let coll_name = collection.name();
    db.run_command(
        mongodb::bson::doc! {
            "collStats": coll_name
        },
        None,
    ).await
}

