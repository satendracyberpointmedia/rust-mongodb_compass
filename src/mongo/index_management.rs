use mongodb::{Collection, Database, bson::Document, IndexModel};
use mongodb::options::{IndexOptions, CreateIndexOptions};
use serde_json::Value;

pub async fn create_index(
    collection: Collection<Document>,
    keys: Document,
    options: Option<IndexOptions>,
) -> mongodb::error::Result<String> {
    let index_model = IndexModel::builder()
        .keys(keys)
        .options(options)
        .build();
    
    let index_name = collection
        .create_index(index_model, None)
        .await?;
    
    Ok(index_name)
}

pub async fn create_index_with_options(
    collection: Collection<Document>,
    keys: Document,
    name: Option<String>,
    unique: Option<bool>,
    sparse: Option<bool>,
    background: Option<bool>,
    expire_after_seconds: Option<i64>,
    partial_filter: Option<Document>,
    text_index_version: Option<i32>,
    default_language: Option<String>,
) -> mongodb::error::Result<String> {
    let mut index_options = IndexOptions::default();
    
    if let Some(name_val) = name {
        index_options.name = Some(name_val);
    }
    
    if let Some(unique_val) = unique {
        index_options.unique = Some(unique_val);
    }
    
    if let Some(sparse_val) = sparse {
        index_options.sparse = Some(sparse_val);
    }
    
    if let Some(background_val) = background {
        index_options.background = Some(background_val);
    }
    
    if let Some(ttl) = expire_after_seconds {
        index_options.expire_after = Some(std::time::Duration::from_secs(ttl as u64));
    }
    
    if let Some(filter) = partial_filter {
        index_options.partial_filter_expression = Some(filter);
    }
    
    if let Some(version) = text_index_version {
        index_options.text_index_version = Some(version);
    }
    
    if let Some(lang) = default_language {
        index_options.default_language = Some(lang);
    }
    
    let index_model = IndexModel::builder()
        .keys(keys)
        .options(index_options)
        .build();
    
    let index_name = collection
        .create_index(index_model, None)
        .await?;
    
    Ok(index_name)
}

pub async fn drop_index(
    collection: Collection<Document>,
    index_name: String,
) -> mongodb::error::Result<()> {
    collection.drop_index(index_name, None).await?;
    Ok(())
}

pub async fn drop_all_indexes(
    collection: Collection<Document>,
) -> mongodb::error::Result<()> {
    collection.drop_indexes(None).await?;
    Ok(())
}

pub async fn rebuild_indexes(
    collection: Collection<Document>,
) -> mongodb::error::Result<()> {
    let db = collection.database();
    let coll_name = collection.name();
    
    db.run_command(
        mongodb::bson::doc! {
            "reIndex": coll_name
        },
        None,
    ).await?;
    
    Ok(())
}

pub async fn get_index_usage_stats(
    database: Database,
    collection_name: String,
) -> mongodb::error::Result<Document> {
    database.run_command(
        mongodb::bson::doc! {
            "aggregate": collection_name,
            "pipeline": [
                {
                    "$indexStats": {}
                }
            ],
            "cursor": {}
        },
        None,
    ).await
}

pub async fn analyze_index_usage(
    collection: Collection<Document>,
) -> mongodb::error::Result<Vec<Document>> {
    let db = collection.database();
    let coll_name = collection.name();
    
    let stats = get_index_usage_stats(db, coll_name.to_string()).await?;
    
    // Extract the cursor results
    if let Some(cursor_doc) = stats.get_document("cursor").ok() {
        if let Some(first_batch) = cursor_doc.get_array("firstBatch").ok() {
            let mut results = Vec::new();
            for item in first_batch {
                if let Ok(doc) = item.as_document() {
                    results.push(doc.clone());
                }
            }
            return Ok(results);
        }
    }
    
    Ok(Vec::new())
}

pub async fn get_index_recommendations(
    collection: Collection<Document>,
    sample_size: Option<usize>,
) -> mongodb::error::Result<Vec<Document>> {
    // This is a simplified version - in production, you'd analyze query patterns
    // For now, we'll return common recommendations based on collection stats
    
    let indexes = crate::mongo::index::list_indexes(collection.clone()).await?;
    let stats = crate::mongo::performance::get_collection_stats(collection.clone()).await?;
    
    let mut recommendations = Vec::new();
    
    // Check if _id index exists (it always does, but check others)
    let has_id_index = indexes.iter().any(|idx| {
        idx.get_str("name").unwrap_or("") == "_id_"
    });
    
    // Recommend compound indexes for common patterns
    // This is a placeholder - real recommendations would analyze query history
    if !has_id_index {
        recommendations.push(mongodb::bson::doc! {
            "type": "missing",
            "field": "_id",
            "recommendation": "Ensure _id index exists (should be automatic)"
        });
    }
    
    Ok(recommendations)
}

