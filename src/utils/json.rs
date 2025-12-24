use mongodb::bson::{self, Document};
use serde_json::Value;

/// Convert BSON Document → JSON Value
pub fn bson_to_json(doc: Document) -> Result<Value, String> {
    let bson_value = bson::to_bson(&doc)
        .map_err(|e| format!("Failed to convert Document to BSON: {}", e))?;
    
    serde_json::to_value(bson_value)
        .map_err(|e| format!("Failed to convert BSON to JSON: {}", e))
}

/// Convert JSON Value → BSON Document
pub fn json_to_bson(value: Value) -> Result<Document, String> {
    // First convert JSON to BSON value
    let bson_value = bson::to_bson(&value)
        .map_err(|e| format!("Failed to convert JSON to BSON value: {}", e))?;
    
    // Then convert BSON value to Document
    match bson_value {
        bson::Bson::Document(doc) => Ok(doc),
        _ => Err("JSON value must be an object to convert to Document".to_string()),
    }
}
