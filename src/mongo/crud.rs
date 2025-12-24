use mongodb::{Collection, bson::Document, options::{InsertManyOptions, UpdateOptions, DeleteOptions}};
use anyhow::Result;

pub async fn insert_one(
    collection: Collection<Document>,
    document: Document,
) -> mongodb::error::Result<mongodb::results::InsertOneResult> {
    collection.insert_one(document, None).await
}

pub async fn insert_many(
    collection: Collection<Document>,
    documents: Vec<Document>,
    ordered: Option<bool>,
) -> mongodb::error::Result<mongodb::results::InsertManyResult> {
    let mut options = InsertManyOptions::default();
    if let Some(ordered_val) = ordered {
        options.ordered = Some(ordered_val);
    }
    collection.insert_many(documents, Some(options)).await
}

pub async fn update_one(
    collection: Collection<Document>,
    filter: Document,
    update: Document,
    upsert: Option<bool>,
) -> mongodb::error::Result<mongodb::results::UpdateResult> {
    let mut options = UpdateOptions::default();
    if let Some(upsert_val) = upsert {
        options.upsert = Some(upsert_val);
    }
    collection.update_one(filter, update, Some(options)).await
}

pub async fn update_many(
    collection: Collection<Document>,
    filter: Document,
    update: Document,
    upsert: Option<bool>,
) -> mongodb::error::Result<mongodb::results::UpdateResult> {
    let mut options = UpdateOptions::default();
    if let Some(upsert_val) = upsert {
        options.upsert = Some(upsert_val);
    }
    collection.update_many(filter, update, Some(options)).await
}

pub async fn delete_one(
    collection: Collection<Document>,
    filter: Document,
) -> mongodb::error::Result<mongodb::results::DeleteResult> {
    collection.delete_one(filter, None).await
}

pub async fn delete_many(
    collection: Collection<Document>,
    filter: Document,
) -> mongodb::error::Result<mongodb::results::DeleteResult> {
    collection.delete_many(filter, None).await
}

pub async fn replace_one(
    collection: Collection<Document>,
    filter: Document,
    replacement: Document,
    upsert: Option<bool>,
) -> mongodb::error::Result<mongodb::results::UpdateResult> {
    let mut options = UpdateOptions::default();
    if let Some(upsert_val) = upsert {
        options.upsert = Some(upsert_val);
    }
    collection.replace_one(filter, replacement, Some(options)).await
}

