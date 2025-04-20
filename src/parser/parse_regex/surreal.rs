use log::{ info, warn, error, debug };
use regex::Regex;
use serde_json::Value;

use crate::parser::parse_regex::clean_html_in_value;

pub fn parse_surreal(chunk: &str) -> Option<Vec<Value>> {
    info!("Using parse method: Surreal (Revised)");
    let mut records = Vec::new();
    let insert_re = Regex::new(r"INSERT(?:\s+INTO\s+([A-Za-z0-9_]+))?\s*\[(?s)(.*)\]\s*;").ok()?;

    for cap in insert_re.captures_iter(chunk) {
        let table_name = cap.get(1).map_or("unknown_table", |m| m.as_str());
        let array_body = cap.get(2)?.as_str();
        let object_splitter_re = Regex::new(r",\s*\{").unwrap();
        let raw_parts: Vec<&str> = object_splitter_re.split(array_body.trim()).collect();
        let mut items = Vec::new();

        for (i, part) in raw_parts.into_iter().enumerate() {
            let mut obj_str = part.trim().to_string();

            if i > 0 && !obj_str.starts_with('{') {
                obj_str.insert(0, '{');
            }

            if !obj_str.starts_with('{') {
                debug!("Prepending missing '{{' to part: {}", obj_str);
                obj_str.insert(0, '{');
            }
            if !obj_str.ends_with('}') {
                debug!("Appending missing '}}' to part: {}", obj_str);
                obj_str.push('}');
            }

            if obj_str.starts_with('{') && obj_str.ends_with('}') {
                items.push(obj_str);
            } else {
                warn!("Skipping malformed part after split/reconstruction: {}", obj_str);
            }
        }

        for obj_str in items {
            debug!("Processing raw object string: {}", obj_str);

            let id_re = Regex::new(r#"(?i)(?:,\s*)?['"]?id['"]?\s*:\s*[^,}]+,?"#).unwrap();
            let no_id = id_re.replace_all(&obj_str, "").to_string();
            debug!("After removing ID: '{}'", no_id);

            let cleaned = no_id
                .trim_matches(|c| (c == ',' || c == ' '))
                .replace(",,", ",")
                .replace("{,", "{")
                .replace(",}", "}");

            let mut cleaned = cleaned.to_string();
            if !cleaned.starts_with('{') && !cleaned.is_empty() {
                cleaned.insert(0, '{');
            }
            if !cleaned.ends_with('}') && !cleaned.is_empty() {
                cleaned.push('}');
            }

            if cleaned == "{}" || cleaned.trim().is_empty() {
                warn!("Skipping record, became empty after cleanup: {}", obj_str);
                continue;
            }
            debug!("After cleanup: '{}'", cleaned);

            let key_quote_re = Regex::new(r#"(?P<k>\b[a-zA-Z_][a-zA-Z0-9_]*\b)\s*:"#).unwrap();
            let quoted_keys = key_quote_re.replace_all(&cleaned, r#""$k":"#).to_string();
            debug!("After quoting keys: '{}'", quoted_keys);

            let swap_quotes = quoted_keys.replace('\'', "\"");
            debug!("After swapping quotes: '{}'", swap_quotes);

            let float_f_re = Regex::new(r#"(\d+\.\d+)f\b"#).unwrap();
            let norm_floats = float_f_re.replace_all(&swap_quotes, "$1").to_string();
            debug!("After normalizing floats: '{}'", norm_floats);

            let final_json_str = norm_floats;
            debug!("Attempting to parse final JSON string: '{}'", final_json_str);

            match serde_json::from_str::<serde_json::Map<String, Value>>(&final_json_str) {
                Ok(mut obj) => {
                    obj.insert("table".into(), Value::String(table_name.into()));
                    let mut v = Value::Object(obj);
                    clean_html_in_value(&mut v);
                    records.push(v);
                    info!("Successfully parsed record for table '{}'", table_name);
                }
                Err(e) => {
                    error!(
                        "Surreal JSON parse error: {} | Failed on string: `{}` | Original object string: `{}`",
                        e,
                        final_json_str,
                        obj_str
                    );
                }
            }
        }
    }

    if records.is_empty() {
        warn!("No records were successfully parsed from the SurrealDB chunk.");
        None
    } else {
        info!("Successfully parsed {} records from SurrealDB chunk.", records.len());
        Some(records)
    }
}
