use log::{ info, warn, debug };
use regex::Regex;
use serde_json::Value;

use crate::parser::parse_regex::{ clean_html_in_value, parse_array };

pub fn parse_mysql(chunk: &str) -> Option<Vec<Value>> {
    info!("Using parse method: MySQL");
    let mut records = Vec::new();
    let insert_re = Regex::new(
        r#"(?is)INSERT INTO\s+[`'\"]?(\w+)[`'\"]?\s*(?:\(([^)]+)\))?\s*VALUES\s*(.*?);"#
    ).ok()?;

    let row_re = Regex::new(r"\((.*?)\)").ok()?;

    for cap in insert_re.captures_iter(chunk) {
        let table = cap.get(1)?.as_str();
        let column_names: Vec<String> = if let Some(cols_match) = cap.get(2) {
            cols_match
                .as_str()
                .split(',')
                .map(|c|
                    c
                        .trim()
                        .trim_matches(&['`', '\'', '"'][..])
                        .to_string()
                )
                .collect()
        } else {
            Vec::new()
        };

        let values_str = cap.get(3)?.as_str();
        let mut inferred_column_count = 0;
        let mut first_row_processed = false;

        for row_cap in row_re.captures_iter(values_str) {
            let row = row_cap.get(1)?.as_str();
            let mut fields = Vec::new();
            let mut current = String::new();
            let mut in_string = false;
            let mut escape_next = false;

            for c in row.chars() {
                if escape_next {
                    current.push(c);
                    escape_next = false;
                } else if c == '\\' {
                    current.push(c);
                    escape_next = true;
                } else if c == '\'' {
                    current.push(c);
                    in_string = !in_string;
                } else if c == ',' && !in_string {
                    fields.push(current.trim().to_string());
                    current.clear();
                } else {
                    current.push(c);
                }
            }
            if !current.is_empty() {
                fields.push(current.trim().to_string());
            }

            let col_names = if !column_names.is_empty() {
                column_names.clone()
            } else if !first_row_processed {
                first_row_processed = true;
                inferred_column_count = fields.len();

                let mut default_cols = Vec::with_capacity(fields.len());
                for i in 0..fields.len() {
                    let col_name = match i {
                        0 => "id".to_string(),
                        1 => "name".to_string(),
                        2 => "description".to_string(),
                        _ => format!("column{}", i),
                    };
                    default_cols.push(col_name);
                }
                default_cols
            } else {
                (0..inferred_column_count)
                    .map(|i| {
                        match i {
                            0 => "id".to_string(),
                            1 => "name".to_string(),
                            2 => "description".to_string(),
                            _ => format!("column{}", i),
                        }
                    })
                    .collect()
            };

            let mut obj = serde_json::Map::new();
            obj.insert("table".to_string(), Value::String(table.to_string()));

            for (i, val_str) in fields.iter().enumerate() {
                if i >= col_names.len() {
                    warn!("More values than columns for table '{}', value: '{}'", table, val_str);
                    continue;
                }

                let mut value = Value::Null;

                if val_str == "NULL" {
                } else if val_str.starts_with('\'') && val_str.ends_with('\'') {
                    let inner_str = val_str.trim_matches('\'');
                    let unescaped_mysql_str = inner_str
                        .replace("''", "'")
                        .replace("\\\\", "\\")
                        .replace("\\'", "'");

                    if
                        (unescaped_mysql_str.starts_with('[') &&
                            unescaped_mysql_str.ends_with(']')) ||
                        (unescaped_mysql_str.starts_with('{') && unescaped_mysql_str.ends_with('}'))
                    {
                        let potential_json_str = unescaped_mysql_str.replace("\\\"", "\"");
                        match serde_json::from_str::<Value>(&potential_json_str) {
                            Ok(json_value) => {
                                value = json_value;
                            }
                            Err(_) => {
                                value = Value::String(unescaped_mysql_str);
                            }
                        }
                    } else {
                        value = Value::String(unescaped_mysql_str);
                    }
                } else if val_str.starts_with('{') && val_str.ends_with('}') {
                    match serde_json::from_str::<Value>(val_str) {
                        Ok(json_val) => {
                            value = json_val;
                        }
                        Err(_) => {
                            value = parse_array(val_str).unwrap_or(
                                Value::String(val_str.to_string())
                            );
                        }
                    }
                } else if val_str.starts_with('[') && val_str.ends_with(']') {
                    match serde_json::from_str::<Value>(val_str) {
                        Ok(json_val) => {
                            value = json_val;
                        }
                        Err(_) => {
                            value = Value::String(val_str.to_string());
                        }
                    }
                } else if let Ok(n) = val_str.parse::<i64>() {
                    value = Value::Number(n.into());
                } else if let Ok(f) = val_str.parse::<f64>() {
                    value = Value::Number(
                        serde_json::Number::from_f64(f).unwrap_or_else(|| (0).into())
                    );
                } else {
                    value = Value::String(val_str.to_string());
                }

                obj.insert(col_names[i].clone(), value);
            }

            let id_key = obj
                .keys()
                .find(|k| k.eq_ignore_ascii_case("id"))
                .cloned();
            if let Some(key) = id_key {
                obj.remove(&key);
                debug!("Removed 'id' field (key: {}) from MySQL record", key);
            }

            if obj.len() > 1 {
                let mut final_value = Value::Object(obj);
                clean_html_in_value(&mut final_value);
                records.push(final_value);
            } else {
                warn!("Skipping MySQL record for table '{}', too few fields after processing", table);
            }
        }
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}
