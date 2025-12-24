use serde_json::Value;
use std::collections::HashMap;

pub fn to_csv(documents: &[Value], headers: Option<Vec<String>>) -> Result<String, String> {
    if documents.is_empty() {
        return Ok(String::new());
    }

    // Extract headers from first document if not provided
    let header_list = if let Some(h) = headers {
        h
    } else {
        extract_keys(&documents[0])
    };

    let mut csv = String::new();
    
    // Write headers
    csv.push_str(&header_list.join(","));
    csv.push('\n');

    // Write rows
    for doc in documents {
        let mut row = Vec::new();
        for header in &header_list {
            let value = doc.get(header)
                .map(|v| format_value_for_csv(v))
                .unwrap_or_else(|| String::new());
            row.push(escape_csv_field(&value));
        }
        csv.push_str(&row.join(","));
        csv.push('\n');
    }

    Ok(csv)
}

fn extract_keys(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => {
            let mut keys = Vec::new();
            extract_keys_recursive(map, &mut keys, String::new());
            keys.sort();
            keys
        }
        _ => Vec::new(),
    }
}

fn extract_keys_recursive(map: &serde_json::Map<String, Value>, keys: &mut Vec<String>, prefix: String) {
    for (key, value) in map {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };

        match value {
            Value::Object(nested_map) => {
                extract_keys_recursive(nested_map, keys, full_key);
            }
            _ => {
                if !keys.contains(&full_key) {
                    keys.push(full_key);
                }
            }
        }
    }
}

fn format_value_for_csv(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| format_value_for_csv(v)).collect();
            format!("[{}]", items.join(";"))
        }
        Value::Object(_) => serde_json::to_string(value).unwrap_or_else(|_| String::new()),
    }
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

pub fn to_json(documents: &[Value], pretty: bool) -> Result<String, String> {
    if pretty {
        serde_json::to_string_pretty(documents)
            .map_err(|e| format!("Failed to serialize to JSON: {}", e))
    } else {
        serde_json::to_string(documents)
            .map_err(|e| format!("Failed to serialize to JSON: {}", e))
    }
}

