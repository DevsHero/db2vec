use log::{ info, warn, debug };
use regex::Regex;
use serde_json::Value;
use crate::parser::parse_regex::clean_html_in_value;
use crate::cli::Args;
use crate::util::exclude::Excluder;

pub fn parse_oracle(content: &str, args: &Args) -> Option<Vec<Value>> {
    info!("Using parse method: Oracle");
    let mut records = Vec::new();

    let excluder = if args.use_exclude {
        Some(Excluder::load("config/exclude.json"))
    } else {
        None
    };

    let insert_re = Regex::new(
        r#"(?is)Insert\s+into\s+([\w\.\"]+)\s+\(([^)]+)\)\s+values\s+\(([^;]+)\);"#
    ).ok()?;

    for cap in insert_re.captures_iter(content) {
        let full_table = cap.get(1)?.as_str();

        let table = (
            if full_table.contains('.') {
                full_table.split('.').last().unwrap_or(full_table)
            } else {
                full_table
            }
        ).trim_matches('"');

        if let Some(ref excl) = excluder {
            if excl.ignore_table(table) {
                info!("Skipping excluded Oracle table: {}", table);
                continue;
            }
        }

        debug!("Processing Oracle INSERT for table: {}", table);

        let columns: Vec<&str> = cap
            .get(2)?
            .as_str()
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .collect();

        let values_str = cap.get(3)?.as_str();
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut function_depth = 0;
        let mut chars = values_str.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '\'' if !in_string => {
                    current.push('\'');
                    in_string = true;
                }
                '\'' if in_string => {
                    if chars.peek() == Some(&'\'') {
                        current.push('\'');
                        current.push('\'');
                        chars.next();
                    } else {
                        current.push('\'');
                        in_string = false;
                    }
                }
                '(' => {
                    current.push('(');
                    if !in_string {
                        function_depth += 1;
                    }
                }
                ')' => {
                    current.push(')');
                    if !in_string && function_depth > 0 {
                        function_depth -= 1;
                    }
                }
                ',' if !in_string && function_depth == 0 => {
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
            let parsed_value = parse_oracle_value(val);
            obj.insert(col.to_string(), parsed_value);
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
            warn!("Skipping Oracle record for table '{}', too few fields after processing", table);
        }
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}

fn parse_oracle_value(val_str: &str) -> Value {
    if val_str.eq_ignore_ascii_case("NULL") {
        return Value::Null;
    }
    if val_str.starts_with('\'') && val_str.ends_with('\'') {
        let inner_str = &val_str[1..val_str.len() - 1].replace("''", "'");

        if
            (inner_str.starts_with('{') && inner_str.ends_with('}')) ||
            (inner_str.starts_with('[') && inner_str.ends_with(']'))
        {
            if let Ok(json_val) = serde_json::from_str(inner_str) {
                return json_val;
            }
        }

        return Value::String(inner_str.to_string());
    }

    if val_str.starts_with("to_timestamp(") {
        let timestamp_re = Regex::new(r"to_timestamp\('([^']+)'").ok();
        if let Some(re) = timestamp_re {
            if let Some(cap) = re.captures(val_str) {
                if let Some(date_match) = cap.get(1) {
                    return Value::String(date_match.as_str().to_string());
                }
            }
        }
        return Value::String("timestamp_parse_error".to_string());
    }

    if let Ok(i) = val_str.parse::<i64>() {
        return Value::Number(i.into());
    }

    if let Ok(f) = val_str.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
    }

    Value::String(val_str.to_string())
}
