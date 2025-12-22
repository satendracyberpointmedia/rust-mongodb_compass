use mongodb::bson::{self, Document};
use serde_json::Value;

/// Convert BSON Document → JSON Value
pub fn bson_to_json(doc: Document) -> Value {
    bson::to_bson(&doc)
        .map(|b| serde_json::to_value(b).unwrap())
        .unwrap_or(Value::Null)
}

/// Convert JSON Value → BSON Document
pub fn json_to_bson(value: Value) -> Result<Document, String> {
    bson::from_json(value.to_string().as_str())
        .map_err(|e| e.to_string())
}
