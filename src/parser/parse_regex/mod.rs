pub mod mysql;
pub mod postgres;
pub mod oracle;
pub mod surreal;
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
