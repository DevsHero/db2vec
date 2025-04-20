use log::{ info, warn, debug };
use regex::Regex;
use serde_json::Value;

use crate::parser::parse_regex::{ clean_html_in_value, parse_array };

pub fn parse_postgres(content: &str) -> Option<Vec<Value>> {
    info!("Using parse method: Postgres");
    let mut records = Vec::new();
    let copy_re = Regex::new(
        r"COPY\s+public\.([a-zA-Z0-9_]+)\s*\(([^)]+)\)\s+FROM stdin;\n((?s:.*?))\n\\\."
    ).ok()?;

    for cap in copy_re.captures_iter(content) {
        let table = cap.get(1)?.as_str();
        let columns: Vec<&str> = cap
            .get(2)?
            .as_str()
            .split(',')
            .map(|s| s.trim())
            .collect();
        let rows = cap.get(3)?.as_str();

        for line in rows.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != columns.len() {
                warn!(
                    "Warning: Mismatched number of columns ({}) and values ({}) for table '{}' in COPY data. Line: '{}'",
                    columns.len(),
                    fields.len(),
                    table,
                    line
                );
                continue;
            }
            let mut obj = serde_json::Map::new();
            obj.insert("table".to_string(), Value::String(table.to_string()));

            for (col, val_str) in columns.iter().zip(fields.iter()) {
                let value = if *val_str == r"\N" {
                    Value::Null
                } else if
                    (val_str.starts_with('{') && val_str.ends_with('}')) ||
                    (val_str.starts_with('[') && val_str.ends_with(']'))
                {
                    match serde_json::from_str::<Value>(val_str) {
                        Ok(json_val) => json_val,
                        Err(_) => {
                            if val_str.starts_with('{') && val_str.ends_with('}') {
                                parse_array(val_str).unwrap_or(Value::String(val_str.to_string()))
                            } else {
                                Value::String(val_str.to_string())
                            }
                        }
                    }
                } else {
                    let unescaped_val = val_str
                        .replace("\\\\", "\\")
                        .replace("\\t", "\t")
                        .replace("\\n", "\n");
                    Value::String(unescaped_val)
                };
                obj.insert(col.trim().to_string(), value);
            }

            let id_key = obj
                .keys()
                .find(|k| k.eq_ignore_ascii_case("id"))
                .cloned();
            if let Some(key) = id_key {
                obj.remove(&key);
                debug!("Removed 'id' field (key: {}) from Postgres record", key);
            }

            if obj.len() > 1 {
                let mut final_value = Value::Object(obj);
                clean_html_in_value(&mut final_value);
                records.push(final_value);
            } else {
                warn!(
                    "Skipping Postgres record for table '{}', became empty after removing ID. Original line: '{}'",
                    table,
                    line
                );
            }
        }
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}
