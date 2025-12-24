use mongodb::{Collection, Database, Client, bson::Document, change_stream::ChangeStream};

pub async fn watch_collection(
    collection: Collection<Document>,
    filter: Option<Document>,
    _operation_types: Option<Vec<String>>,
) -> mongodb::error::Result<ChangeStream<Document>> {
    use mongodb::options::ChangeStreamOptions;
    
    let mut options = ChangeStreamOptions::default();
    
    // Set full document option for better change event details
    options.full_document = Some(mongodb::options::FullDocument::UpdateLookup);
    
    if let Some(filter_doc) = filter {
        collection.watch_with_options(vec![filter_doc], options).await
    } else {
        collection.watch_with_options(vec![], options).await
    }
}

pub async fn watch_database(
    database: Database,
    filter: Option<Document>,
    _operation_types: Option<Vec<String>>,
) -> mongodb::error::Result<ChangeStream<Document>> {
    use mongodb::options::ChangeStreamOptions;
    
    let mut options = ChangeStreamOptions::default();
    options.full_document = Some(mongodb::options::FullDocument::UpdateLookup);
    
    if let Some(filter_doc) = filter {
        database.watch_with_options(vec![filter_doc], options).await
    } else {
        database.watch_with_options(vec![], options).await
    }
}

pub async fn watch_client(
    client: &mongodb::Client,
    filter: Option<Document>,
    _operation_types: Option<Vec<String>>,
) -> mongodb::error::Result<ChangeStream<Document>> {
    use mongodb::options::ChangeStreamOptions;
    
    let mut options = ChangeStreamOptions::default();
    options.full_document = Some(mongodb::options::FullDocument::UpdateLookup);
    
    if let Some(filter_doc) = filter {
        client.watch_with_options(vec![filter_doc], options).await
    } else {
        client.watch_with_options(vec![], options).await
    }
}

