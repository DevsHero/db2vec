pub mod db;
pub mod parser;
pub mod embedding;
pub mod cli;
use std::io::{ stdout, Cursor, Read, Write };
use std::time::Instant;
use clap::Parser;
use crate::parser::detect_format;
use db::redis::RedisDatabase;
use db::{ ChromaDatabase, PineconeDatabase };
use db::Database;
use db::MilvusDatabase;
use db::QdrantDatabase;
use db::SurrealDatabase;
use embedding::embeding::generate_embedding;
use parser::parse_database_export;
use cli::Args;
use dotenvy::dotenv;
use log::{ info, error };
use encoding_rs::UTF_16LE;
use encoding_rs_io::DecodeReaderBytesBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off")).init();
    println!(
        r#"
        ____  ____  ____  _  _  ____   ___ 
        (    \(  _ \(___ \/ )( \(  __) / __)
         ) D ( ) _ ( / __/\ \/ / ) _) ( (__ 
        (____/(____/(____) \__/ (____) \___)                                                                      
        "#
    );
    println!("Database to Vector Migration Tool\n");

    let args = Args::parse();
    let file_path = args.data_file.clone();
    let export_type = args.db_export_type.clone();
    let clone_export_type = export_type.clone();

    info!("Reading file: {}", file_path);

    let raw = std::fs::read(&file_path)?;
    let content = if raw.starts_with(&[0xff, 0xfe]) {
        let mut decoder = DecodeReaderBytesBuilder::new()
            .encoding(Some(UTF_16LE))
            .bom_override(true)
            .build(Cursor::new(raw));
        let mut s = String::new();
        decoder.read_to_string(&mut s)?;
        s
    } else {
        String::from_utf8(raw)?
    };

    info!("Detecting format...");
    let format = detect_format(&file_path, &content);

    info!("Processing {} format file: {}", format, file_path);
    info!("Parsing records...");

    let records = match parse_database_export(&content, &format, &args) {
        Ok(recs) => recs,
        Err(e) => {
            error!("Error parsing database export: {}", e);
            return Err(e);
        }
    };

    let total_records = records.len();
    info!("Successfully parsed {} records", total_records);

    let database: Box<dyn Database> = match clone_export_type.as_str() {
        "redis" => Box::new(RedisDatabase::new(&args)?),
        "qdrant" => Box::new(QdrantDatabase::new(&args)?),
        "chroma" => Box::new(ChromaDatabase::new(&args)?),
        "milvus" => Box::new(MilvusDatabase::new(&args)?),
        "surreal" => Box::new(SurrealDatabase::new(&args)?),
        "pinecone" => Box::new(PineconeDatabase::new(&args)?),
        _ => {
            return Err("Unsupported database type".into());
        }
    };

    let start_time = Instant::now();
    let mut processed_count = 0;
    let spinner_chars = ['|', '/', '-', '\\'];

    println!("Starting migration...");

    for record in records {
        let id = uuid::Uuid::new_v4();
        let table = record
            .get("table")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let record_text = serde_json::to_string(&record)?;
        let vector = generate_embedding(&record_text)?;
        database.store_vector(table, &id.to_string(), &vector, &record)?;
        processed_count += 1;
        let spinner_char = spinner_chars[processed_count % spinner_chars.len()];
        print!("\rProcessing... {} ", spinner_char);
        stdout().flush()?;
    }

    print!("\r{}", " ".repeat(16));
    let elapsed_time = start_time.elapsed();
    println!(
        "\rFinished processing {} records in {:.2} seconds",
        total_records,
        elapsed_time.as_secs_f64()
    );
    println!("Migration Complete.");
    Ok(())
}
