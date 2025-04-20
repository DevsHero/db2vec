use log::{ info, warn, debug };
use regex::Regex;
use serde_json::Value;

use crate::parser::parse_regex::clean_html_in_value;

pub fn parse_oracle(content: &str) -> Option<Vec<Value>> {
    info!("Using parse method: Oracle");
    let mut records = Vec::new();
    let insert_re = Regex::new(r#"Insert into ([\w\.]+) \(([^)]+)\) values \(([^)]+)\);"#).ok()?;

    for cap in insert_re.captures_iter(content) {
        let table = cap.get(1)?.as_str();
        let columns: Vec<&str> = cap
            .get(2)?
            .as_str()
            .split(',')
            .map(|s| s.trim())
            .collect();
        let values_str = cap.get(3)?.as_str();
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut chars = values_str.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\'' if !in_string => {
                    in_string = true;
                }
                '\'' if in_string => {
                    if chars.peek() == Some(&'\'') {
                        current.push('\'');
                        chars.next();
                    } else {
                        in_string = false;
                    }
                }
                ',' if !in_string => {
                    fields.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(c),
            }
        }
        if !current.is_empty() || fields.len() < columns.len() {
            fields.push(current.trim().to_string());
        }

        if fields.len() != columns.len() {
            warn!(
                "Mismatched number of columns ({}) and values ({}) for table '{}'. Row values: '{}'",
                columns.len(),
                fields.len(),
                table,
                values_str
            );
            continue;
        }

        let mut obj = serde_json::Map::new();
        obj.insert("table".to_string(), Value::String(table.to_string()));
        for (col, val) in columns.iter().zip(fields.iter()) {
            let value = if val == "NULL" {
                Value::Null
            } else if let Ok(n) = val.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(f) = val.parse::<f64>() {
                Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| (0).into()))
            } else {
                Value::String(val.to_string())
            };
            obj.insert(col.trim().to_string(), value);
        }

        let id_key = obj
            .keys()
            .find(|k| k.eq_ignore_ascii_case("id"))
            .cloned();
        if let Some(key) = id_key {
            obj.remove(&key);
            debug!("Removed 'id' field (key: {}) from Oracle record", key);
        }

        if obj.len() > 1 {
            let mut final_value = Value::Object(obj);
            clean_html_in_value(&mut final_value);
            records.push(final_value);
        } else {
            warn!(
                "Skipping Oracle record for table '{}', became empty after removing ID. Original values: '{}'",
                table,
                values_str
            );
        }
    }
    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}
