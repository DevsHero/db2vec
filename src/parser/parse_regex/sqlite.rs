use log::{ info, warn, debug };
use regex::Regex;
use serde_json::Value;

use crate::parser::parse_regex::clean_html_in_value;

pub fn parse_sqlite(chunk: &str) -> Option<Vec<Value>> {
    info!("Using parse method: SQLite");
    let mut records = Vec::new();
    let create_re = Regex::new(
        r"(?is)CREATE TABLE\s+(?:IF NOT EXISTS\s+)?(?:`?(\w+)`?|(\w+))\s*\((.*?)\);"
    ).ok()?;

    let column_def_re = Regex::new(r"^\s*(?:`?(\w+)`?|(\w+))\s+").ok()?;
    let mut table_columns = std::collections::HashMap::new();

    for cap in create_re.captures_iter(chunk) {
        let table_name = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str());
        let cols_def_match = cap.get(3);

        if let (Some(table_name), Some(cols_def_match)) = (table_name, cols_def_match) {
            let cols_def = cols_def_match.as_str();
            let mut cols = Vec::new();
            for line in cols_def.lines() {
                let trimmed_line = line.trim();
                if
                    trimmed_line.starts_with("--") ||
                    trimmed_line.starts_with("PRIMARY") ||
                    trimmed_line.starts_with("UNIQUE") ||
                    trimmed_line.starts_with("CHECK") ||
                    trimmed_line.starts_with("FOREIGN") ||
                    trimmed_line.is_empty()
                {
                    continue;
                }
                if let Some(col_cap) = column_def_re.captures(trimmed_line) {
                    if let Some(col_name) = col_cap.get(1).or_else(|| col_cap.get(2)) {
                        cols.push(col_name.as_str().to_string());
                    }
                }
            }
            if !cols.is_empty() {
                debug!("Found columns for table '{}': {:?}", table_name, cols);
                table_columns.insert(table_name.to_string(), cols);
            }
        }
    }

    if table_columns.is_empty() {
        warn!("Could not parse any CREATE TABLE statements to find column names in SQLite chunk.");
        return None;
    }

    let insert_re = Regex::new(
        r"(?is)INSERT INTO\s+(?:`?(\w+)`?|(\w+))\s+VALUES\s*\((.*?)\);"
    ).ok()?;

    for cap in insert_re.captures_iter(chunk) {
        let table = match cap.get(1).or_else(|| cap.get(2)) {
            Some(t) => t.as_str(),
            None => {
                continue;
            }
        };

        if table == "sqlite_sequence" {
            continue;
        }

        let columns = match table_columns.get(table) {
            Some(cols) => cols,
            None => {
                warn!("Skipping INSERT for table '{}' because columns were not found (CREATE TABLE missing or unparsed).", table);
                continue;
            }
        };
        let values_str = cap.get(3).map_or("", |m| m.as_str());
        let mut fields = Vec::new();
        let mut current_field = String::new();
        let mut in_string = false;
        let mut chars = values_str.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\'' {
                if in_string && chars.peek() == Some(&'\'') {
                    current_field.push(c);
                    chars.next();
                } else {
                    in_string = !in_string;
                }
                current_field.push(c);
            } else if c == ',' && !in_string {
                fields.push(current_field.trim().to_string());
                current_field.clear();
            } else {
                current_field.push(c);
            }
        }

        fields.push(current_field.trim().to_string());

        if fields.len() != columns.len() {
            warn!(
                "Mismatched number of columns ({}) and values ({}) for table '{}'. Row: '{}'",
                columns.len(),
                fields.len(),
                table,
                values_str
            );
            continue;
        }

        let mut obj = serde_json::Map::new();
        obj.insert("table".to_string(), Value::String(table.to_string()));

        for (i, col) in columns.iter().enumerate() {
            let val_str = &fields[i];
            let mut value = Value::Null;
            if val_str == "NULL" {
            } else if val_str.starts_with('\'') && val_str.ends_with('\'') && val_str.len() >= 2 {
                let inner_str = &val_str[1..val_str.len() - 1];
                let unescaped_str = inner_str.replace("''", "'");

                if
                    (unescaped_str.starts_with('[') && unescaped_str.ends_with(']')) ||
                    (unescaped_str.starts_with('{') && unescaped_str.ends_with('}'))
                {
                    match serde_json::from_str::<Value>(&unescaped_str) {
                        Ok(json_value) => {
                            value = json_value;
                        }
                        Err(_) => {
                            value = Value::String(unescaped_str);
                        }
                    }
                } else {
                    value = Value::String(unescaped_str);
                }
            } else if let Ok(n) = val_str.parse::<i64>() {
                value = Value::Number(n.into());
            } else if let Ok(f) = val_str.parse::<f64>() {
                value = Value::Number(
                    serde_json::Number::from_f64(f).unwrap_or_else(|| (0).into())
                );
            } else {
                warn!(
                    "Unrecognized value format for column '{}' in table '{}': {}",
                    col,
                    table,
                    val_str
                );
                value = Value::String(val_str.to_string());
            }
            obj.insert(col.clone(), value);
        }

        let id_key = obj
            .keys()
            .find(|k| k.eq_ignore_ascii_case("id"))
            .cloned();
        if let Some(key) = id_key {
            obj.remove(&key);
            debug!("Removed 'id' field (key: {}) from SQLite record", key);
        }

        if obj.len() > 1 {
            let mut final_value = Value::Object(obj);
            clean_html_in_value(&mut final_value);
            records.push(final_value);
        } else {
            warn!(
                "Skipping SQLite record for table '{}', became empty after removing ID. Original values: '{}'",
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
