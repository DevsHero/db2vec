use regex::Regex;
use log::{ info, warn };
use serde_json::Value;
use crate::parser::parse_regex::clean_html_in_value;

pub fn parse_surreal(chunk: &str) -> Option<Vec<Value>> {
    info!("Using parse method: Surreal");
    let mut records = Vec::new();
    let insert_re = Regex::new(r"INSERT(?:\s+INTO\s+([a-zA-Z0-9_]+))?\s*\[(?s)(.*)\]\s*;").ok()?;

    for insert_cap in insert_re.captures_iter(chunk) {
        let table_name = insert_cap.get(1).map_or("unknown_table", |m| m.as_str());
        let array_content = insert_cap.get(2)?.as_str();
        let object_re = Regex::new(r"\}\s*,\s*\{").unwrap();
        let items: Vec<String> = object_re
            .split(array_content)
            .map(|s| {
                let trimmed = s.trim();
                let mut obj = trimmed.to_string();
                if !obj.starts_with('{') {
                    obj.insert(0, '{');
                }
                if !obj.ends_with('}') {
                    obj.push('}');
                }
                obj
            })
            .collect();

        for item_str in items {
            if let Ok(mut obj) = serde_json::from_str::<serde_json::Map<String, Value>>(&item_str) {
                obj.insert("table".to_string(), Value::String(table_name.to_string()));
                let mut value = Value::Object(obj);
                clean_html_in_value(&mut value);
                records.push(value);
                continue;
            }
            let mut record = serde_json::Map::new();
            let kv_regex = Regex::new(
                r#"([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*('[^']*'|\[.*?\]|\{.*?\}|[0-9.]+(?:f)?|true|false|null)"#
            ).unwrap();

            for caps in kv_regex.captures_iter(&item_str) {
                let key = caps.get(1).unwrap().as_str();
                let raw_val = caps.get(2).unwrap().as_str().trim();

                let value = if raw_val.starts_with('[') && raw_val.ends_with(']') {
                    serde_json
                        ::from_str::<Value>(raw_val)
                        .unwrap_or(Value::String(raw_val.to_string()))
                } else if raw_val.starts_with('{') && raw_val.ends_with('}') {
                    serde_json
                        ::from_str::<Value>(raw_val)
                        .unwrap_or(Value::String(raw_val.to_string()))
                } else if raw_val.starts_with('\'') && raw_val.ends_with('\'') {
                    Value::String(raw_val.trim_matches('\'').to_string())
                } else if let Ok(n) = raw_val.trim_end_matches('f').parse::<f64>() {
                    if n.fract() == 0.0 {
                        Value::Number((n as i64).into())
                    } else {
                        serde_json::Number
                            ::from_f64(n)
                            .map(Value::Number)
                            .unwrap_or(Value::String(raw_val.to_string()))
                    }
                } else if raw_val == "true" {
                    Value::Bool(true)
                } else if raw_val == "false" {
                    Value::Bool(false)
                } else if raw_val == "null" {
                    Value::Null
                } else {
                    Value::String(raw_val.to_string())
                };

                record.insert(key.to_string(), value);
            }

            record.insert("table".to_string(), Value::String(table_name.to_string()));
            record.remove("id");

            if record.len() > 1 {
                let mut value = Value::Object(record);
                clean_html_in_value(&mut value);
                records.push(value);
            } else {
                warn!("Regex fallback produced empty record for: {}", item_str);
            }
        }
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}
