use log::{ info, warn, debug };
use regex::Regex;
use serde_json::Value;

use crate::parser::{ parse_array, parse_regex::clean_html_in_value };

pub fn parse_mysql(chunk: &str) -> Option<Vec<Value>> {
    info!("Using parse method: MySQL");
    let mut records = Vec::new();

    let create_re = Regex::new(
        r"(?is)CREATE TABLE\s+(?:IF NOT EXISTS\s+)?[`']?(\w+)[`']?\s*\((.*?)\)\s*(?:ENGINE=|AUTO_INCREMENT=|DEFAULT CHARSET=|COLLATE=|COMMENT=|$)"
    ).ok()?;

    let column_def_re = Regex::new(r"^\s*[`']?(\w+)[`']?").ok()?;
    let mut table_columns = std::collections::HashMap::new();
    for cap in create_re.captures_iter(chunk) {
        if let (Some(table_match), Some(cols_def_match)) = (cap.get(1), cap.get(2)) {
            let table_name = table_match.as_str();
            let cols_def = cols_def_match.as_str();
            let mut cols = Vec::new();
            for line in cols_def.lines() {
                if let Some(col_cap) = column_def_re.captures(line.trim()) {
                    if let Some(col_name) = col_cap.get(1) {
                        let name_upper = col_name.as_str().to_uppercase();
                        if
                            name_upper != "PRIMARY" &&
                            name_upper != "KEY" &&
                            name_upper != "CONSTRAINT" &&
                            name_upper != "UNIQUE" &&
                            name_upper != "FULLTEXT" &&
                            name_upper != "SPATIAL" &&
                            name_upper != "FOREIGN" &&
                            !name_upper.starts_with("INDEX")
                        {
                            cols.push(col_name.as_str().to_string());
                        }
                    }
                }
            }
            if !cols.is_empty() {
                table_columns.insert(table_name.to_string(), cols);
            }
        }
    }

    if table_columns.is_empty() {
        warn!("Could not parse any CREATE TABLE statements to find column names.");
    }

    let insert_re = Regex::new(
        r"(?is)INSERT INTO\s+[`']?(\w+)[`']?\s*(?:\([^)]+\))?\s*VALUES\s*(.*?);"
    ).ok()?;

    let row_re = Regex::new(r"\((.*?)\)").ok()?;

    for cap in insert_re.captures_iter(chunk) {
        let table = cap.get(1)?.as_str();
        let columns = match table_columns.get(table) {
            Some(cols) => cols,
            None => {
                warn!("Skipping INSERT for table '{}' because columns were not found.", table);
                continue;
            }
        };
        let values_str = cap.get(2)?.as_str();
        for row_cap in row_re.captures_iter(values_str) {
            let row = row_cap.get(1)?.as_str();
            let mut fields = Vec::new();
            let mut current = String::new();
            let mut in_string = false;
            let mut escape_next = false;
            let mut chars = row.chars().peekable();

            while let Some(c) = chars.next() {
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
            if !current.is_empty() || fields.len() < columns.len() {
                fields.push(current.trim().to_string());
            }

            if fields.len() != columns.len() {
                warn!(
                    "Mismatched number of columns ({}) and values ({}) for table '{}'. Row: '{}'",
                    columns.len(),
                    fields.len(),
                    table,
                    row
                );
                continue;
            }

            let mut obj = serde_json::Map::new();
            obj.insert("table".to_string(), Value::String(table.to_string()));

            for (i, col) in columns.iter().enumerate() {
                let val_str = &fields[i];
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
                obj.insert(col.clone(), value);
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
                warn!(
                    "Skipping MySQL record for table '{}', became empty after removing ID. Original row: '{}'",
                    table,
                    row
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
