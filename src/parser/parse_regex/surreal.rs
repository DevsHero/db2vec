use regex::Regex;
use log::{ info, warn, debug };
use serde_json::Value;
use crate::parser::parse_regex::clean_html_in_value;
use crate::cli::Args;
use crate::util::exclude::Excluder;

pub fn parse_surreal(chunk: &str, args: &Args) -> Option<Vec<Value>> {
    info!("Using parse method: Surreal");
    let mut records = Vec::new();

    let excluder = if args.use_exclude {
        Some(Excluder::load("config/exclude.json"))
    } else {
        None
    };

    let table_header_re = Regex::new(r"--\s*TABLE DATA:\s*([a-zA-Z0-9_]+)").ok()?;
    let insert_re = Regex::new(r"INSERT\s*\[(?s)(.*?)\]\s*;").ok()?;

    let mut inserts = Vec::new();
    for insert_cap in insert_re.captures_iter(chunk) {
        if let Some(array_content) = insert_cap.get(1) {
            let array_text = array_content.as_str();
            let full_match = insert_cap.get(0).unwrap().as_str();
            inserts.push((full_match, array_text));
        }
    }

    if inserts.is_empty() {
        warn!("No INSERT statements found in chunk");
        return None;
    }

    let mut table_sections = Vec::new();
    for table_cap in table_header_re.captures_iter(chunk) {
        if let Some(table_name) = table_cap.get(1) {
            let pos = table_cap.get(0).unwrap().start();
            table_sections.push((table_name.as_str().to_string(), pos));
        }
    }

    table_sections.sort_by_key(|&(_, pos)| pos);

    for (i, (insert_stmt, array_content)) in inserts.iter().enumerate() {
        let insert_pos = chunk.find(insert_stmt).unwrap_or(0);
        let mut table_name = "unknown_table".to_string();
        for (t_name, t_pos) in &table_sections {
            if *t_pos < insert_pos {
                table_name = t_name.clone();
            } else {
                break;
            }
        }

        if let Some(ref excl) = excluder {
            if excl.ignore_table(&table_name) {
                info!("Skipping excluded table: {}", table_name);
                continue; 
            }
        }

        info!("Processing INSERT #{} for table: {}", i, table_name);
        debug!("Parsing data from table {}: {:.100}...", table_name, array_content);

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
                obj.insert("table".to_string(), Value::String(table_name.clone()));
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

            record.insert("table".to_string(), Value::String(table_name.clone()));
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
        warn!("No records parsed from section");
        None
    } else {
   
        info!("Successfully parsed {} records", records.len());
        Some(records)
    }
}
