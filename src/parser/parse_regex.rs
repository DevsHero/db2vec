use regex::Regex;
use serde_json::Value;

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

pub fn parse_mysql(chunk: &str) -> Option<Vec<Value>> {
    println!("Using parse method: MySQL");
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
        println!("Warning: Could not parse any CREATE TABLE statements to find column names.");
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
                println!("Warning: Skipping INSERT for table '{}' because columns were not found.", table);
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
                println!(
                    "Warning: Mismatched number of columns ({}) and values ({}) for table '{}'. Row: '{}'",
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
                            value = parse_postgres_array(val_str).unwrap_or(
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

pub fn parse_postgres(content: &str) -> Option<Vec<Value>> {
    println!("Using parse method: Postgres");
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
                println!(
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
                                parse_postgres_array(val_str).unwrap_or(
                                    Value::String(val_str.to_string())
                                )
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
                obj.insert(col.to_string(), value);
            }
            let mut value = Value::Object(obj);
            clean_html_in_value(&mut value);
            records.push(value);
        }
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}

fn parse_postgres_array(array_str: &str) -> Option<Value> {
    let content = array_str.get(1..array_str.len() - 1)?;
    if content.is_empty() {
        return Some(Value::Array(vec![]));
    }
    let mut elements = Vec::new();
    let mut current_element = String::new();
    let mut chars = content.chars().peekable();
    let mut in_quotes = false;
    let mut escape_next = false;

    while let Some(c) = chars.next() {
        if escape_next {
            current_element.push(c);
            escape_next = false;
        } else if c == '\\' {
            escape_next = true;
        } else if c == '"' {
            in_quotes = !in_quotes;
        } else if c == ',' && !in_quotes {
            elements.push(Value::String(current_element.trim().to_string()));
            current_element.clear();
        } else {
            current_element.push(c);
        }
    }

    elements.push(Value::String(current_element.trim().to_string()));

    Some(Value::Array(elements))
}

pub fn parse_oracle(content: &str) -> Option<Vec<Value>> {
    println!("Using parse method: Oracle");
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
        if !current.is_empty() {
            fields.push(current.trim().to_string());
        }

        let mut obj = serde_json::Map::new();
        obj.insert("table".to_string(), Value::String(table.to_string()));
        for (col, val) in columns.iter().zip(fields.iter()) {
            let value = if val == "NULL" {
                Value::Null
            } else if val.starts_with('\'') && val.ends_with('\'') {
                Value::String(val.trim_matches('\'').replace("''", "'"))
            } else if let Ok(n) = val.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(f) = val.parse::<f64>() {
                Value::Number(serde_json::Number::from_f64(f).unwrap())
            } else {
                Value::String(val.clone())
            };
            obj.insert(col.to_string(), value);
        }
        let mut value = Value::Object(obj);
        clean_html_in_value(&mut value);
        records.push(value);
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}

pub fn extract_json_array(text: &str) -> Option<&str> {
    let open_bracket = text.find('[')?;
    let mut depth = 1;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, c) in text[open_bracket + 1..].chars().enumerate() {
        match c {
            '[' if !in_string => {
                depth += 1;
            }
            ']' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[open_bracket..=open_bracket + i + 1]);
                }
            }
            '"' if !escape_next => {
                in_string = !in_string;
            }
            '\\' if in_string && !escape_next => {
                escape_next = true;
            }
            _ => {
                escape_next = false;
            }
        }
    }

    None
}

fn clean_html_in_value(val: &mut Value) {
    match val {
        Value::String(s) => {
            if s.contains('<') && s.contains('>') {
                *s = html2text
                    ::from_read(s.as_bytes(), 9999)
                    .unwrap_or_default()
                    .replace('\n', " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
            }
        }
        Value::Array(arr) => {
            for v in arr {
                clean_html_in_value(v);
            }
        }
        Value::Object(obj) => {
            for v in obj.values_mut() {
                clean_html_in_value(v);
            }
        }
        _ => {}
    }
}
