use log::{ debug, info, warn };
use regex::Regex;
use serde_json::Value;
use crate::parser::parse_regex::clean_html_in_value;
use crate::cli::Args;
use crate::util::exclude::Excluder;

pub fn parse_mssql(chunk: &str, args: &Args) -> Option<Vec<Value>> {
    info!("Using parse method: MSSQL");
    let mut records = Vec::new();
    
    let excluder = if args.use_exclude {
        Some(Excluder::load("config/exclude.json"))
    } else {
        None
    };

    let insert_re = Regex::new(
        r"(?is)INSERT\s+\[(?:dbo|DB_OWNER)\]\.\[(\w+)\]\s*(?:\((.*?)\))?\s*VALUES\s*"
    ).ok()?;
    let values_re = Regex::new(r"\(((?:[^()]*|\((?:[^()]*|\([^()]*\))*\))*)\)").ok()?;

    for cap in insert_re.captures_iter(chunk) {
        let table = cap.get(1)?.as_str();
        
        if let Some(ref excl) = excluder {
            if excl.ignore_table(table) {
                info!("Skipping excluded MSSQL table: {}", table);
                continue;
            }
        }

        info!("Processing INSERT for MSSQL table: {}", table);

        let column_names: Vec<String> = if let Some(cols_match) = cap.get(2) {
            cols_match
                .as_str()
                .split(',')
                .map(|c|
                    c
                        .trim()
                        .trim_matches(&['[', ']'][..])
                        .to_string()
                )
                .collect()
        } else {
            Vec::new()
        };

        let insert_statement_end = cap.get(0)?.end();
        let search_area = &chunk[insert_statement_end..];

        for values_cap in values_re.captures_iter(search_area) {
            let values_str = values_cap.get(1)?.as_str();
            let mut fields = Vec::new();
            let mut current = String::new();
            let mut in_string = false;
            let mut in_cast = 0;
            let mut escape_next = false;

            for c in values_str.chars() {
                if escape_next {
                    current.push(c);
                    escape_next = false;
                } else if c == '\\' {
                    current.push(c);
                    escape_next = true;
                } else if c == '\'' {
                    current.push(c);
                    if !in_string && current.ends_with("N'") {
                        in_string = true;
                    } else if in_string {
                        if let Some(next_char) = search_area.chars().nth(current.len()) {
                            if next_char == '\'' {
                                continue;
                            }
                        }
                        in_string = false;
                    }
                } else if c == '(' {
                    current.push(c);
                    if current.contains("CAST") {
                        in_cast += 1;
                    }
                } else if c == ')' {
                    current.push(c);
                    if in_cast > 0 {
                        in_cast -= 1;
                    }
                } else if c == ',' && !in_string && in_cast == 0 {
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
            } else {
                (0..fields.len())
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

            if fields.len() != col_names.len() {
                warn!(
                    "Column count mismatch for table '{}': {} columns vs {} values",
                    table,
                    col_names.len(),
                    fields.len()
                );
                continue;
            }

            let mut obj = serde_json::Map::new();
            obj.insert("table".to_string(), Value::String(table.to_string()));

            for (i, val_str) in fields.iter().enumerate() {
                if i >= col_names.len() {
                    continue;
                }

                let value = parse_mssql_value(val_str);
                obj.insert(col_names[i].clone(), value);
            }

            let id_key = obj
                .keys()
                .find(|k| k.eq_ignore_ascii_case("id"))
                .cloned();
            if let Some(key) = id_key {
                obj.remove(&key);
                debug!("Removed 'id' field (key: {}) from MSSQL record", key);
            }

            if obj.len() > 1 {
                let mut final_value = Value::Object(obj);
                clean_html_in_value(&mut final_value);
                records.push(final_value);
            }
        }
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}

fn parse_mssql_value(val_str: &str) -> Value {
    if val_str == "NULL" {
        return Value::Null;
    }
    if val_str.starts_with("N'") && val_str.ends_with("'") {
        let inner_str = &val_str[2..val_str.len() - 1].replace("''", "'");

        if
            (inner_str.starts_with("[") && inner_str.ends_with("]")) ||
            (inner_str.starts_with("{") && inner_str.ends_with("}"))
        {
            if let Ok(json_val) = serde_json::from_str(inner_str) {
                return json_val;
            }
        }

        return Value::String(inner_str.to_string());
    }

    if val_str.starts_with("CAST(") {
        let re = Regex::new(r"CAST\(\s*N?'?(.*?)'?\s+AS").ok();
        if let Some(re) = re {
            if let Some(cap) = re.captures(val_str) {
                if let Some(m) = cap.get(1) {
                    return parse_mssql_value(m.as_str());
                }
            }
        }

        return Value::String(val_str.to_string());
    }

    if val_str == "0" || val_str == "1" {
        if let Ok(b) = val_str.parse::<i8>() {
            return Value::Bool(b != 0);
        }
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
