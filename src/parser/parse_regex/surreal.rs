use once_cell::sync::Lazy;
use regex::Regex;
use log::{ info, warn, error, debug };
use serde_json::Value;
use crate::parser::parse_regex::clean_html_in_value;

static INSERT_RE: Lazy<Regex> = Lazy::new(||
    Regex::new(r"INSERT(?:\s+INTO\s+([A-Za-z0-9_]+))?\s*\[(?s)(.*)\]\s*;").unwrap()
);
static SPLIT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r",\s*\{").unwrap());
static ID_RE: Lazy<Regex> = Lazy::new(||
    Regex::new(r#"(?i)(?:,\s*)?['"]?id['"]?\s*:\s*[^,}]+,?"#).unwrap()
);
static KEY_QUOTE_RE: Lazy<Regex> = Lazy::new(||
    Regex::new(r#"(?P<k>\b[a-zA-Z_][a-zA-Z0-9_]*\b)\s*:"#).unwrap()
);
static FLOAT_F_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(\d+\.\d+)f\b"#).unwrap());

pub fn parse_surreal(chunk: &str) -> Option<Vec<Value>> {
    info!("Using parse method: Surreal (Revised)");
    let mut records = Vec::new();
    for cap in INSERT_RE.captures_iter(chunk) {
        let table = cap
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or("unknown");
        let body = cap.get(2).unwrap().as_str();
        let mut parts = Vec::new();
        for (i, p) in SPLIT_RE.split(body).enumerate() {
            let mut s = p.trim().to_string();
            if i > 0 && !s.starts_with('{') {
                s.insert(0, '{');
            }
            if !s.ends_with('}') {
                s.push('}');
            }
            parts.push(s);
        }
        for obj in parts {
            debug!("raw: {}", obj);
            let no_id = ID_RE.replace_all(&obj, "").to_string();
            let clean = no_id
                .trim_matches(|c| (c == ',' || c == ' '))
                .replace(",,", ",")
                .replace("{,", "{")
                .replace(",}", "}");
            if clean.len() < 2 {
                continue;
            }
            let quoted = KEY_QUOTE_RE.replace_all(&clean, r#""$k":"#);
            let swapped = quoted.replace("'", "\"");
            let norm = FLOAT_F_RE.replace_all(&swapped, "$1");
            match serde_json::from_str::<serde_json::Map<_, _>>(&norm) {
                Ok(mut m) => {
                    m.insert("table".into(), Value::String(table.into()));
                    let mut v = Value::Object(m);
                    clean_html_in_value(&mut v);
                    info!("parsed table={}", table);
                    records.push(v);
                }
                Err(e) => error!("surreal parse failed: {} | `{}`", e, norm),
            }
        }
    }
    if records.is_empty() {
        warn!("no surreal records");
        None
    } else {
        Some(records)
    }
}
