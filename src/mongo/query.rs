use mongodb::{bson::Document, Collection, options::FindOptions};

pub async fn find(
    collection: Collection<Document>,
    filter: Document,
) -> mongodb::error::Result<mongodb::Cursor<Document>> {
    collection.find(filter, None).await
}

pub async fn find_with_options(
    collection: Collection<Document>,
    filter: Document,
    sort: Option<Document>,
    limit: Option<u64>,
    skip: Option<u64>,
    projection: Option<Document>,
) -> mongodb::error::Result<mongodb::Cursor<Document>> {
    let mut options = FindOptions::default();
    
    if let Some(sort_doc) = sort {
        options.sort = Some(sort_doc);
    }
    
    if let Some(limit_val) = limit {
        options.limit = Some(limit_val as i64);
    }
    
    if let Some(skip_val) = skip {
        options.skip = Some(skip_val);
    }
    
    if let Some(projection_doc) = projection {
        options.projection = Some(projection_doc);
    }
    
    collection.find(filter, Some(options)).await
}
