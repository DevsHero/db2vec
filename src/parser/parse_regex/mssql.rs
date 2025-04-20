use log::{ debug, info, warn };
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

use crate::parser::parse_regex::clean_html_in_value;

pub fn parse_mssql(chunk: &str) -> Option<Vec<Value>> {
    info!("Using parse method: MSSQL");
    let mut records = Vec::new();
    let create_re = Regex::new(
        r"(?is)CREATE TABLE\s+(?:\[(?:\w+)\]\.)?\[(\w+)\]\s*\((.*?)\)\s*(?:ON|WITH|TEXTIMAGE_ON|;|\bGO\b)"
    ).expect("Failed to compile MSSQL CREATE TABLE regex");
    let column_def_re = Regex::new(r"^\s*\[(\w+)\]\s+").expect(
        "Failed to compile MSSQL column definition regex"
    );

    let mut table_columns: HashMap<String, Vec<String>> = HashMap::new();

    for cap in create_re.captures_iter(chunk) {
        let table_name = cap.get(1).map(|m| m.as_str());
        let cols_def_match = cap.get(2);

        if let (Some(table_name), Some(cols_def_match)) = (table_name, cols_def_match) {
            let cols_def_raw = cols_def_match.as_str();
            let mut cols = Vec::new();
            for line in cols_def_raw.lines() {
                let trimmed_line = line.trim();
                if
                    trimmed_line.starts_with("PRIMARY") ||
                    trimmed_line.starts_with("CONSTRAINT") ||
                    trimmed_line.starts_with("UNIQUE") ||
                    trimmed_line.starts_with("CHECK") ||
                    trimmed_line.starts_with("FOREIGN")
                {
                    break;
                }
                if trimmed_line.is_empty() || trimmed_line.starts_with("--") {
                    continue;
                }
                if let Some(col_cap) = column_def_re.captures(trimmed_line) {
                    if let Some(col_name) = col_cap.get(1) {
                        cols.push(col_name.as_str().to_string());
                    }
                } else {
                    debug!("Line did not match column definition regex: {}", trimmed_line);
                }
            }
            if !cols.is_empty() {
                debug!("Found columns for table '{}': {:?}", table_name, cols);
                table_columns.insert(table_name.to_string(), cols);
            } else {
                warn!(
                    "Parsed CREATE TABLE for '{}' but extracted no columns from: ```{}```",
                    table_name,
                    cols_def_raw
                );
            }
        }
    }

    if table_columns.is_empty() {
        warn!("Could not parse any CREATE TABLE statements to find column names in MSSQL chunk.");
        return None;
    }

    let insert_start_re = Regex::new(
        r"(?is)INSERT\s+(?:\[\w+\]\.)?\[(\w+)\]\s*(?:\((.*?)\))?\s*VALUES\s*\("
    ).expect("Failed to compile INSERT start regex");

    let mut search_pos = 0;
    while let Some(cap) = insert_start_re.captures(&chunk[search_pos..]) {
        let table = cap.get(1).unwrap().as_str();
        let (insert_cols_str, insert_cols) = match cap.get(2) {
            Some(m) => {
                let raw = m.as_str();
                let cols: Vec<&str> = raw
                    .split(',')
                    .map(|c| c.trim().trim_matches(&['[', ']'][..]))
                    .collect();
                (Some(raw), cols)
            }
            None => {
                let cols = table_columns.get(table).unwrap().iter().map(AsRef::as_ref).collect();
                (None, cols)
            }
        };

        let mat = cap.get(0).unwrap();
        let vals_start = search_pos + mat.end();
        let bytes = chunk.as_bytes();
        let mut depth = 1;
        let mut idx = vals_start;
        while idx < bytes.len() && depth > 0 {
            match bytes[idx] as char {
                '(' => {
                    depth += 1;
                }
                ')' => {
                    depth -= 1;
                }
                _ => {}
            }
            idx += 1;
        }
        if depth != 0 {
            break;
        }
        let vals_end = idx - 1;
        let values_str = &chunk[vals_start..vals_end];
        debug!(
            "INSERT into {} cols={:?} raw_cols={:?} values_str={}",
            table,
            insert_cols,
            insert_cols_str.unwrap_or("<inferred>"),
            values_str
        );

        search_pos = vals_end;
        let mut fields = Vec::new();
        let mut current_pos = 0;
        while current_pos < values_str.len() {
            let remaining_str = &values_str[current_pos..];
            let trimmed_remaining = remaining_str.trim_start_matches(
                |c: char| (c.is_whitespace() || c == ',')
            );

            if trimmed_remaining.is_empty() {
                break;
            }

            current_pos = values_str.len() - trimmed_remaining.len();

            let mut matched_value: Option<String> = None;
            let mut next_pos_delta = 0;
            if trimmed_remaining.starts_with("CAST(") {
                let mut paren_level = 0;
                let mut cast_end = None;
                for (i, c) in trimmed_remaining.char_indices() {
                    if c == '(' {
                        paren_level += 1;
                    } else if c == ')' {
                        paren_level -= 1;
                        if paren_level == 0 {
                            cast_end = Some(i + 1);
                            break;
                        }
                    }
                }
                if let Some(end_idx) = cast_end {
                    let cast_expr = &trimmed_remaining[..end_idx];
                    if
                        let Some(inner_val_match) = Regex::new(r"(?is)CAST\(\s*(.*?)\s+AS")
                            .unwrap()
                            .captures(cast_expr)
                    {
                        if let Some(inner_val) = inner_val_match.get(1) {
                            matched_value = Some(inner_val.as_str().trim().to_string());
                            debug!(
                                "    Parsed complex CAST: '{}', storing inner value: '{}'",
                                cast_expr,
                                matched_value.as_ref().unwrap()
                            );
                        } else {
                            warn!("Could not extract inner value from CAST: {}", cast_expr);
                            matched_value = Some(cast_expr.to_string());
                        }
                    } else {
                        warn!("Could not parse inner value from CAST: {}", cast_expr);
                        matched_value = Some(cast_expr.to_string());
                    }
                    next_pos_delta = end_idx;
                } else {
                    warn!(
                        "Unmatched parenthesis in potential CAST expression starting at: {}",
                        &trimmed_remaining[..(50).min(trimmed_remaining.len())]
                    );

                    if let Some(comma_pos) = trimmed_remaining.find(',') {
                        matched_value = Some(trimmed_remaining[..comma_pos].trim().to_string());
                        next_pos_delta = comma_pos;
                    } else {
                        matched_value = Some(trimmed_remaining.trim().to_string());
                        next_pos_delta = trimmed_remaining.len();
                    }
                }
            } else if trimmed_remaining.starts_with("N'") {
                let mut end_quote = None;
                let mut escaping = false;
                for (i, c) in trimmed_remaining.char_indices().skip(2) {
                    if escaping {
                        escaping = false;
                    } else if c == '\'' {
                        if trimmed_remaining.chars().nth(i + 1) == Some('\'') {
                            escaping = true;
                        } else {
                            end_quote = Some(i + 1);
                            break;
                        }
                    }
                }
                if let Some(end_idx) = end_quote {
                    matched_value = Some(trimmed_remaining[..end_idx].to_string());
                    debug!("    Parsed NString: '{}'", matched_value.as_ref().unwrap());
                    next_pos_delta = end_idx;
                } else {
                    warn!(
                        "Unmatched quote in potential NString starting at: {}",
                        &trimmed_remaining[..(50).min(trimmed_remaining.len())]
                    );
                    if let Some(comma_pos) = trimmed_remaining.find(',') {
                        matched_value = Some(trimmed_remaining[..comma_pos].trim().to_string());
                        next_pos_delta = comma_pos;
                    } else {
                        matched_value = Some(trimmed_remaining.trim().to_string());
                        next_pos_delta = trimmed_remaining.len();
                    }
                }
            } else if trimmed_remaining.starts_with("NULL") {
                if trimmed_remaining.len() == 4 || trimmed_remaining.chars().nth(4) == Some(',') {
                    matched_value = Some("NULL".to_string());
                    debug!("    Parsed NULL");
                    next_pos_delta = 4;
                }
            }

            if matched_value.is_none() {
                let num_re = Regex::new(r"^([-+]?\d+\.?\d*)").unwrap();
                if let Some(num_cap) = num_re.captures(trimmed_remaining) {
                    if let Some(num_match) = num_cap.get(1) {
                        let num_str = num_match.as_str();
                        matched_value = Some(num_str.to_string());
                        debug!("    Parsed Number: '{}'", num_str);
                        next_pos_delta = num_str.len();
                    }
                } else {
                    if let Some(comma_pos) = trimmed_remaining.find(',') {
                        let word = trimmed_remaining[..comma_pos].trim();
                        if !word.is_empty() {
                            matched_value = Some(word.to_string());
                            debug!("    Parsed Other/Fallback: '{}'", word);
                            next_pos_delta = comma_pos;
                        } else {
                            matched_value = Some("".to_string());
                            debug!("    Parsed Empty Value");
                            next_pos_delta = comma_pos;
                        }
                    } else {
                        let word = trimmed_remaining.trim();
                        matched_value = Some(word.to_string());
                        debug!("    Parsed Last Other/Fallback: '{}'", word);
                        next_pos_delta = trimmed_remaining.len();
                    }
                }
            }

            if let Some(val) = matched_value {
                fields.push(val);
                current_pos += next_pos_delta;
            } else {
                warn!(
                    "Failed to parse value at position {}, remaining: '{}'",
                    current_pos,
                    trimmed_remaining
                );
                break;
            }
        }

        debug!(
            "Finished parsing values manually. Parsed fields count: {}. Parsed fields: {:?}",
            fields.len(),
            fields
        );

        if fields.len() != insert_cols.len() {
            warn!(
                "Mismatched number of columns ({}) and parsed values ({}) for table '{}'. Columns: {:?}, Values String: '{}', Parsed: {:?}",
                insert_cols.len(),
                fields.len(),
                table,
                insert_cols,
                values_str,
                fields
            );
            continue;
        }

        let mut obj = serde_json::Map::new();
        obj.insert("table".to_string(), Value::String(table.to_string()));

        for (i, col_name) in insert_cols.iter().enumerate() {
            let val_str: &String = &fields[i];
            let mut value = Value::Null;

            if val_str == "NULL" {
            } else if val_str.starts_with("N'") && val_str.ends_with('\'') && val_str.len() >= 3 {
                let inner_str = &val_str[2..val_str.len() - 1];
                let unescaped_str = inner_str.replace("''", "'");

                if
                    (unescaped_str.starts_with('[') && unescaped_str.ends_with(']')) ||
                    (unescaped_str.starts_with('{') && unescaped_str.ends_with('}'))
                {
                    match serde_json::from_str::<Value>(&unescaped_str) {
                        Ok(json_value) => {
                            value = json_value;
                        }
                        Err(e) => {
                            warn!(
                                "Failed to parse potential JSON string '{}': {}",
                                unescaped_str,
                                e
                            );
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
                    serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0))
                );
            } else if val_str == "1" || val_str == "0" {
                value = Value::Bool(val_str == "1");
            } else {
                let cleaned_val = val_str
                    .trim_start_matches("N'")
                    .trim_end_matches('\'')
                    .replace("''", "'");

                if let Ok(n) = cleaned_val.parse::<i64>() {
                    value = Value::Number(n.into());
                } else if let Ok(f) = cleaned_val.parse::<f64>() {
                    value = Value::Number(
                        serde_json::Number
                            ::from_f64(f)
                            .unwrap_or_else(|| serde_json::Number::from(0))
                    );
                } else {
                    warn!(
                        "Using string fallback for unrecognized/CAST value format for column '{}' in table '{}': Original='{}', Cleaned='{}'",
                        col_name,
                        table,
                        val_str,
                        cleaned_val
                    );
                    value = Value::String(cleaned_val);
                }
            }
            obj.insert(col_name.to_string(), value);
        }
        debug!("Built JSON object before ID removal for table '{}': {:?}", table, obj);

        let id_key = obj
            .keys()
            .find(|k| k.eq_ignore_ascii_case("id"))
            .cloned();
        if let Some(key) = id_key {
            obj.remove(&key);
            debug!("Removed 'id' field (key: {}) from MSSQL record", key);
        }
        debug!("Built JSON object after ID removal for table '{}': {:?}", table, obj);

        if obj.len() > 1 {
            let mut final_value = Value::Object(obj);
            clean_html_in_value(&mut final_value);
            debug!("Adding final record to results: {:?}", final_value);
            records.push(final_value);
        } else {
            warn!(
                "Skipping record for table '{}', became empty after removing ID or contained only ID. Original values: '{}', Object after ID removal: {:?}",
                table,
                values_str,
                obj
            );
        }
    }

    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}
