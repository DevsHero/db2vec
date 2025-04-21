use once_cell::sync::Lazy;
use regex::Regex;
use log::{ info, warn, error, debug };
use serde_json::Value;
use crate::parser::parse_regex::clean_html_in_value;

pub fn parse_surreal(chunk: &str) -> Option<Vec<Value>> {
    println!("Using parse method: Surreal");
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
                if !trimmed.starts_with('{') && !trimmed.ends_with('}') {
                    if !trimmed.starts_with('{') {
                        format!("{{{}", trimmed)
                    } else {
                        trimmed.to_string()
                    }
                } else if !trimmed.ends_with('}') {
                    format!("{}}}", trimmed)
                } else {
                    trimmed.to_string()
                }
            })
            .collect();

        for item_str in items {
            match serde_json::from_str::<serde_json::Map<String, Value>>(&item_str) {
                Ok(mut obj) => {
                    obj.insert("table".to_string(), Value::String(table_name.to_string()));
                    let mut value = Value::Object(obj);
                    clean_html_in_value(&mut value);
                    records.push(value);
                    continue;
                }
                Err(e) => {
                    eprintln!("JSON parsing failed for item: {}, Error: {}", item_str, e);
                }
            }

            let mut record = serde_json::Map::new();
            let kv_regex = Regex::new(
                r#"([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*('[^']*'|\[.*?\]|\{.*?\}|[0-9.]+(?:f)?|true|false|null)"#
            ).ok()?;

            for caps in kv_regex.captures_iter(&item_str) {
                if caps.len() < 3 {
                    continue;
                }
                let key = caps.get(1).unwrap().as_str();
                let raw_val = caps.get(2).unwrap().as_str().trim();

                let value = if raw_val.starts_with('[') && raw_val.ends_with(']') {
                    match serde_json::from_str::<Value>(raw_val) {
                        Ok(v) => v,
                        Err(_) => Value::String(raw_val.to_string()),
                    }
                } else if raw_val.starts_with('{') && raw_val.ends_with('}') {
                    match serde_json::from_str::<Value>(raw_val) {
                        Ok(v) => v,
                        Err(_) => Value::String(raw_val.to_string()),
                    }
                } else if raw_val.starts_with('\'') && raw_val.ends_with('\'') {
                    Value::String(raw_val.trim_matches('\'').to_string())
                } else if let Ok(num) = raw_val.trim_end_matches('f').parse::<f64>() {
                    if num.fract() == 0.0 {
                        Value::Number((num as i64).into())
                    } else {
                        Value::Number(
                            serde_json::Number::from_f64(num).unwrap_or_else(|| (0).into())
                        )
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

            if record.len() > 2 {
                let mut value = Value::Object(record);
                clean_html_in_value(&mut value);
                records.push(value);
            } else {
                println!("Warning: Regex fallback resulted in empty record for: {}", item_str);
            }
        }
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}
