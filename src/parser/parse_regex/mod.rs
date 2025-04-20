pub mod mysql;
pub mod postgres;
pub mod oracle;
pub mod surreal;
pub mod sqlite;
pub mod mssql;
use serde_json::Value;

pub fn clean_html_in_value(val: &mut Value) {
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
pub fn parse_array(array_str: &str) -> Option<Value> {
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
