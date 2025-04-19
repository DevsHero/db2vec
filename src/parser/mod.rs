use parse_regex::{ parse_mysql, parse_oracle, parse_postgres, parse_surreal };
use crate::cli::Args;

use serde_json::Value;
use std::error::Error;
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

    println!("Found {} chunks to process", chunks.len());
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

                println!("Parsed {} records from chunk {} using regex", records.len(), i);
                if args.debug {
                    for (j, rec) in records.iter().enumerate() {
                        println!("Debug: Record {} in chunk {}: {}", j, i, rec); // Adjusted message slightly
                    }
                }

                all_records.extend(records);
            }
            None => {
                println!("Regex parsing failed for chunk {}", i);
            }
        }
    }

    println!("Total records parsed: {}", all_records.len());

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
