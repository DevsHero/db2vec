use crate::cli::Args;

use parse_regex::{
    mysql::parse_mysql,
    oracle::parse_oracle,
    postgres::parse_postgres,
    surreal::parse_surreal,
};
use serde_json::Value;
use std::error::Error;
use log::{ info, warn, debug };
pub mod parse_regex;
// pub mod parse_ai;
pub trait ExportParser {
    fn parse(&self, content: &str) -> Result<Vec<Value>, Box<dyn Error>>;
}

pub fn parse_database_export(
    content: &str,
    format: &str,
    args: &Args
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let chunks: Vec<&str> = content
        .split("INSERT [")
        .filter(|s| !s.trim().is_empty())
        .collect();

    info!("Found {} chunks to process", chunks.len());
    let mut all_records = Vec::new();
    let mut last_table = "default".to_string();

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_text = if i > 0 { format!("INSERT [{}", chunk) } else { chunk.to_string() };
        if format == "surreal" {
            if
                let Some(table_caps) = regex::Regex
                    ::new(r"TABLE DATA:\s*([a-zA-Z0-9_]+)")
                    .ok()
                    .and_then(|re| re.captures(&chunk_text))
            {
                last_table = table_caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| last_table.clone());
            }
        }

        match parse_with_regex(&chunk_text, format) {
            Some(mut records) => {
                if format == "surreal" {
                    for record in records.iter_mut() {
                        if let Some(obj) = record.as_object_mut() {
                            obj.insert(
                                "table".to_string(),
                                serde_json::Value::String(last_table.clone())
                            );
                        }
                    }
                }

                info!("Parsed {} records from chunk {} using regex", records.len(), i);
                if args.debug {
                    for (j, rec) in records.iter().enumerate() {
                        debug!("Debug: Record {} in chunk {}: {}", j, i, rec);
                    }
                }

                all_records.extend(records);
            }
            None => {
                warn!("Regex parsing failed for chunk {}", i);
            }
        }
    }

    info!("Total records parsed: {}", all_records.len());

    Ok(all_records)
}

pub fn detect_format(file_path: &str, content: &str) -> String {
    let _content_lower = content.to_lowercase();

    // SurrealDB distinctive patterns
    if file_path.ends_with(".surql") {
        return "surreal".to_string();
    }

    // Oracle distinctive patterns
    if
        content.contains("REM INSERTING into") ||
        content.contains("SET DEFINE OFF;") ||
        content.contains("Insert into ") ||
        (content.contains("CREATE TABLE \"") &&
            content.contains("PCTFREE") &&
            content.contains("TABLESPACE")) ||
        content.contains("BUFFER_POOL DEFAULT FLASH_CACHE DEFAULT CELL_FLASH_CACHE DEFAULT") ||
        content.contains("USING INDEX PCTFREE") ||
        content.contains("ALTER SESSION SET EVENTS") ||
        content.contains("DBMS_LOGREP_IMP")
    {
        return "oracle".to_string();
    }

    // PostgreSQL distinctive patterns
    if
        content.contains("COPY public.") ||
        content.contains("OWNER TO") ||
        content.contains("SET standard_conforming_strings") ||
        content.contains("pg_catalog.setval")
    {
        return "postgres".to_string();
    }

    // MySQL distinctive patterns
    if
        content.contains("ENGINE=InnoDB") ||
        content.contains("LOCK TABLES") ||
        content.contains("/*!40") ||
        content.contains("AUTO_INCREMENT") ||
        content.contains("COLLATE=utf8mb4")
    {
        return "mysql".to_string();
    }

    // Default fallback
    "json".to_string()
}

pub fn parse_with_regex(chunk: &str, format: &str) -> Option<Vec<Value>> {
    match format {
        "surreal" => parse_surreal(chunk),
        "mysql" => parse_mysql(chunk),
        "postgres" => parse_postgres(chunk),
        "oracle" => parse_oracle(chunk),
        _ => None,
    }
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
