use crate::cli::Args;

use log::{ info, warn, debug };
use parse_regex::mssql::parse_mssql;
use parse_regex::mysql::parse_mysql;
use parse_regex::oracle::parse_oracle;
use parse_regex::postgres::parse_postgres;
use parse_regex::sqlite::parse_sqlite;
use parse_regex::surreal::parse_surreal;
use serde_json::Value;
use std::error::Error;

pub mod parse_regex;
pub trait ExportParser {
    fn parse(&self, content: &str) -> Result<Vec<Value>, Box<dyn Error>>;
}

pub fn parse_database_export(
    content: &str,
    format: &str,
    args: &Args
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let mut all_records = Vec::new();
    let chunks: Vec<String> = match format {
        "mssql" | "postgres" | "mysql" | "surreal" | "sqlite" => {
            info!("Processing {} file without chunking", format);
            vec![content.to_string()]
        }
        "oracle" => {
            content
                .split("Insert into")
                .filter(|s| !s.trim().is_empty())
                .enumerate()
                .map(|(i, s)| {
                    if i > 0 { format!("Insert into{}", s) } else { s.to_string() }
                })
                .collect()
        }
        _ => {
            warn!("Using default (single chunk) processing for unknown format: {}", format);
            vec![content.to_string()]
        }
    };

    info!("Found {} chunks to process for format '{}'", chunks.len(), format);

    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.trim().is_empty() {
            debug!("Skipping empty chunk {}", i);
            continue;
        }

        match parse_with_regex(&chunk, format) {
            Some(records) => {
                if !records.is_empty() {
                    info!("Parsed {} records from chunk {}", records.len(), i);
                    if args.debug {
                        for (j, rec) in records.iter().enumerate() {
                            debug!("Debug: Record {} in chunk {}: {}", j, i, rec);
                        }
                    }
                    all_records.extend(records);
                } else {
                    debug!("Regex parsing yielded 0 records for chunk {}", i);
                }
            }
            None => {
                warn!("Regex parsing failed entirely for chunk {}", i);
                if args.debug && chunk.len() < 1000 {
                    debug!("Content of failed chunk {}:\n{}", i, chunk);
                } else if args.debug {
                    debug!(
                        "Content of failed chunk {} (truncated):\n{}...",
                        i,
                        &chunk[..std::cmp::min(chunk.len(), 1000)]
                    );
                }
            }
        }
    }

    info!("Total records parsed: {}", all_records.len());
    Ok(all_records)
}

pub fn detect_format(file_path: &str, content: &str) -> String {
    let _content_lower = content.to_lowercase();

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
        (content.contains("COPY ") && content.contains(" FROM stdin;")) ||
        content.contains("PostgreSQL database dump") ||
        (content.contains("SET ") && content.contains("standard_conforming_strings")) ||
        content.contains("ALTER TABLE ONLY") ||
        (content.contains("CREATE TYPE") && content.contains("AS ENUM")) ||
        (content.contains("CREATE SEQUENCE") && content.contains("OWNED BY"))
    {
        return "postgres".to_string();
    }

    // SQLite distinctive patterns
    if
        content.starts_with("PRAGMA foreign_keys=OFF;") ||
        (content.contains("BEGIN TRANSACTION;") &&
            content.contains("COMMIT;") &&
            content.contains("CREATE TABLE") &&
            !content.contains("ENGINE=InnoDB") &&
            !content.contains("TABLESPACE") &&
            content.contains("INSERT INTO ")) ||
        content.contains("sqlite_sequence")
    {
        return "sqlite".to_string();
    }

    // MSSQL distinctive patterns
    if
        content.contains("SET ANSI_NULLS ON") ||
        content.contains("SET QUOTED_IDENTIFIER ON") ||
        content.contains("CREATE TABLE [dbo].") ||
        content.contains("INSERT [dbo].") ||
        content.contains("WITH (PAD_INDEX = OFF") ||
        content.contains("GO")
    {
        return "mssql".to_string();
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

    "json".to_string()
}

pub fn parse_with_regex(chunk: &str, format: &str) -> Option<Vec<Value>> {
    match format {
        "surreal" => parse_surreal(chunk),
        "mysql" => parse_mysql(chunk),
        "postgres" => parse_postgres(chunk),
        "oracle" => parse_oracle(chunk),
        "sqlite" => parse_sqlite(chunk),
        "mssql" => parse_mssql(chunk),
        _ => None,
    }
}
