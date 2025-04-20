use log::{ info, warn, error, debug };
use regex::Regex;
use serde_json::Value;

pub fn parse_surreal(chunk: &str) -> Option<Vec<Value>> {
    info!("Using parse method: Surreal (Revised)"); // Added "(Revised)" for clarity
    let mut records = Vec::new();
    // Regex to capture the table name and the array content within INSERT [...]
    let insert_re = Regex::new(r"INSERT(?:\s+INTO\s+([A-Za-z0-9_]+))?\s*\[(?s)(.*)\]\s*;").ok()?;

    for cap in insert_re.captures_iter(chunk) {
        let table_name = cap.get(1).map_or("unknown_table", |m| m.as_str());
        let array_body = cap.get(2)?.as_str();

        // Split the array body into potential object strings
        // Split *before* the opening brace of subsequent objects: ", {"
        let object_splitter_re = Regex::new(r",\s*\{").unwrap();
        let raw_parts: Vec<&str> = object_splitter_re.split(array_body.trim()).collect();
        let mut items = Vec::new();

        for (i, part) in raw_parts.into_iter().enumerate() {
            let mut obj_str = part.trim().to_string();

            // The first part should already start with '{' from the original array body
            // Parts after the first split need '{' prepended because the split removed it
            if i > 0 && !obj_str.starts_with('{') {
                obj_str.insert(0, '{');
            }

            // Ensure the string looks like a complete object, defensively adding braces if needed.
            // This primarily ensures the very first object starts correctly and all objects end correctly.
            if !obj_str.starts_with('{') {
                debug!("Prepending missing '{{' to part: {}", obj_str);
                obj_str.insert(0, '{');
            }
            if !obj_str.ends_with('}') {
                debug!("Appending missing '}}' to part: {}", obj_str);
                obj_str.push('}');
            }

            // Basic validation: Check if it looks like an object
            if obj_str.starts_with('{') && obj_str.ends_with('}') {
                items.push(obj_str);
            } else {
                warn!("Skipping malformed part after split/reconstruction: {}", obj_str);
            }
        }

        for obj_str in items {
            // Now loop through the reconstructed items
            debug!("Processing raw object string: {}", obj_str);

            // 1) Strip entire id: field (robust regex)
            let id_re = Regex::new(r#"(?i)(?:,\s*)?['"]?id['"]?\s*:\s*[^,}]+,?"#).unwrap();
            let no_id = id_re.replace_all(&obj_str, "").to_string();
            debug!("After removing ID: '{}'", no_id); // Log with quotes for clarity

            // 2) Clean up aggressively after ID removal
            let cleaned = no_id
                .trim_matches(|c| (c == ',' || c == ' ')) // Remove leading/trailing commas/spaces
                .replace(",,", ",") // Collapse double commas
                .replace("{,", "{") // Fix comma after opening brace
                .replace(",}", "}"); // Fix comma before closing brace

            // Ensure it still looks like an object if content remains
            let mut cleaned = cleaned.to_string();
            if !cleaned.starts_with('{') && !cleaned.is_empty() {
                cleaned.insert(0, '{');
            }
            if !cleaned.ends_with('}') && !cleaned.is_empty() {
                cleaned.push('}');
            }

            // Skip if the object became empty
            if cleaned == "{}" || cleaned.trim().is_empty() {
                warn!("Skipping record, became empty after cleanup: {}", obj_str);
                continue;
            }
            debug!("After cleanup: '{}'", cleaned); // Log with quotes

            // 3) Quote keys: foo: → "foo":
            let key_quote_re = Regex::new(r#"(?P<k>\b[a-zA-Z_][a-zA-Z0-9_]*\b)\s*:"#).unwrap();
            let quoted_keys = key_quote_re.replace_all(&cleaned, r#""$k":"#).to_string();
            debug!("After quoting keys: '{}'", quoted_keys); // Log with quotes

            // 4) Swap single → double quotes for string values
            let swap_quotes = quoted_keys.replace('\'', "\"");
            debug!("After swapping quotes: '{}'", swap_quotes); // Log with quotes

            // 5) Strip trailing 'f' from floats
            let float_f_re = Regex::new(r#"(\d+\.\d+)f\b"#).unwrap();
            let norm_floats = float_f_re.replace_all(&swap_quotes, "$1").to_string();
            debug!("After normalizing floats: '{}'", norm_floats); // Log with quotes

            let final_json_str = norm_floats;
            debug!("Attempting to parse final JSON string: '{}'", final_json_str); // Log with quotes

            // 6) Parse exactly once with serde_json
            match serde_json::from_str::<serde_json::Map<String, Value>>(&final_json_str) {
                Ok(mut obj) => {
                    obj.insert("table".into(), Value::String(table_name.into()));
                    let mut v = Value::Object(obj);
                    clean_html_in_value(&mut v); // Assuming clean_html_in_value exists
                    records.push(v);
                    info!("Successfully parsed record for table '{}'", table_name);
                }
                Err(e) => {
                    // Log detailed error if parsing fails
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

            // Populate the object
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
    info!("Using parse method: Oracle");
    let mut records = Vec::new();
    let insert_re = Regex::new(r#"Insert into ([\w\.]+) \(([^)]+)\) values \(([^)]+)\);"#).ok()?;

    for cap in insert_re.captures_iter(content) {
        let table = cap.get(1)?.as_str();
        let columns: Vec<&str> = cap
            .get(2)?
            .as_str()
            .split(',')
            .map(|s| s.trim()) // Trim column names here
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
                    // Don't add the opening quote to the value
                }
                '\'' if in_string => {
                    if chars.peek() == Some(&'\'') {
                        // Handle escaped single quote ''
                        current.push('\'');
                        chars.next(); // Consume the second quote
                    } else {
                        // End of string
                        in_string = false;
                        // Don't add the closing quote to the value
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

        // Add check for column/value count mismatch
        if fields.len() != columns.len() {
            warn!(
                "Mismatched number of columns ({}) and values ({}) for table '{}'. Row values: '{}'",
                columns.len(),
                fields.len(),
                table,
                values_str // Log the original values string for context
            );
            continue;
        }

        let mut obj = serde_json::Map::new();
        obj.insert("table".to_string(), Value::String(table.to_string()));
        for (col, val) in columns.iter().zip(fields.iter()) {
            let value = if val == "NULL" {
                Value::Null
                // Check if it looks like a number *before* checking for quotes
            } else if let Ok(n) = val.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(f) = val.parse::<f64>() {
                Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| (0).into()))
                // Now handle quoted strings (which were already processed by the loop above)
            } else {
                Value::String(val.to_string()) // Value is already unquoted and unescaped
            };
            // Use trimmed column name
            obj.insert(col.trim().to_string(), value);
        }

        // *** ADD THIS BLOCK TO REMOVE THE 'ID' FIELD ***
        let id_key = obj
            .keys()
            .find(|k| k.eq_ignore_ascii_case("id"))
            .cloned();
        if let Some(key) = id_key {
            obj.remove(&key);
            debug!("Removed 'id' field (key: {}) from Oracle record", key);
        }

        // Check if object is not empty (excluding the 'table' field)
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
                    ::from_read(s.as_bytes(), usize::MAX)
                    .unwrap_or_else(|_| s.clone())
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
