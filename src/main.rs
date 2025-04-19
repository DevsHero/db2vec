pub mod db;
pub mod parser;
pub mod embedding;
pub mod cli;

use std::fs;
use clap::Parser;
use crate::parser::detect_format;
use db::redis::RedisDatabase;
use db::ChromaDatabase;
use db::Database;
use db::MilvusDatabase;
use db::QdrantDatabase;
use db::SurrealDatabase;
use embedding::embeding::generate_embedding;
use parser::parse_database_export;
use cli::Args;
use dotenvy::dotenv;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args = Args::parse();
    let file_path = args.data_file.clone();
    let export_type = args.db_export_type.clone();
    let clone_export_type = export_type.clone();
    let content = fs
        ::read_to_string(&file_path)
        .expect(&format!("Unable to read the export file: {}", file_path));
    let format = detect_format(&file_path, &content);

    println!("Processing {} format file: {}", format, file_path);

    let records = match parse_database_export(&content, &format, &args) {
        Ok(recs) => recs,
        Err(e) => {
            eprintln!("Error parsing database export: {}", e);
            return Err(e);
        }
    };

    let clone_records = records.clone();
    println!("Successfully parsed {} records", records.len());

    let database: Box<dyn Database> = match clone_export_type.as_str() {
        "redis" => Box::new(RedisDatabase::new(&args)?),
        "qdrant" => Box::new(QdrantDatabase::new(&args)?),
        "chroma" => Box::new(ChromaDatabase::new(&args)?),
        "milvus" => Box::new(MilvusDatabase::new(&args)?),
        "surreal" => Box::new(SurrealDatabase::new(&args)?),
        _ => {
            return Err("Unsupported database type".into());
        }
    };

    for record in records {
        let id = record
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .unwrap_or_else(|| uuid::Uuid::new_v4());

        let table = record
            .get("table")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let record_text = serde_json::to_string(&record)?;
        let vector = generate_embedding(&record_text)?;

        database.store_vector(table, &id.to_string(), &vector, &record)?;
        println!("Migrated record {} to database", id);
    }

    println!("Migration complete: {} records processed", clone_records.len());
    Ok(())
}
