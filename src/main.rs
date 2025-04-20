pub mod db;
pub mod parser;
pub mod embedding;
pub mod cli;
use std::io::{ stdout, Cursor, Read, Write };
use std::time::Instant;
use std::sync::{ Arc, Mutex };
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::thread;
use std::time::Duration;
use clap::Parser;
use crate::parser::detect_format;
use db::redis::RedisDatabase;
use db::{ ChromaDatabase, PineconeDatabase };
use db::Database;
use db::MilvusDatabase;
use db::QdrantDatabase;
use db::SurrealDatabase;
use db::store_in_batches;
use db::DbError;
use embedding::embeding::{ generate_embedding, generate_embeddings_batch };
use parser::parse_database_export;
use cli::Args;
use dotenvy::dotenv;
use log::{ info, error };
use encoding_rs::UTF_16LE;
use encoding_rs_io::DecodeReaderBytesBuilder;
use rayon::prelude::*;

fn main() -> Result<(), DbError> {
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
            let err_msg = format!("Error parsing database export: {}", e);
            error!("{}", err_msg);
            return Err(err_msg.into());
        }
    };

    let total_records = records.len();
    info!("Successfully parsed {} records", total_records);

    let thread_count = if args.num_threads == 0 { num_cpus::get() } else { args.num_threads };
    rayon::ThreadPoolBuilder::new().num_threads(thread_count).build_global().unwrap();
    info!("Using {} threads for parallel processing", thread_count);

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
    let embedding_count = Arc::new(AtomicUsize::new(0));
    let total_count = total_records;
    let stop_animation = Arc::new(Mutex::new(false));
    let animation_stop = stop_animation.clone();
    let embedding_count_for_animation = embedding_count.clone();
    let animation_handle = thread::spawn(move || {
        let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let mut spinner_idx = 0;

        while !*animation_stop.lock().unwrap() {
            let count = embedding_count_for_animation.load(Ordering::Relaxed);
            spinner_idx = (spinner_idx + 1) % spinner_chars.len();

            print!(
                "\r{} Generating embeddings... [{}/{}] ({}%)",
                spinner_chars[spinner_idx],
                count,
                total_count,
                (count * 100) / total_count.max(1)
            );
            stdout().flush().unwrap();

            thread::sleep(Duration::from_millis(80));
        }
        print!("\r{}\r", " ".repeat(80));
        stdout().flush().unwrap();
    });

    let embedding_counter = embedding_count.clone();
    let chunk_size = args.embedding_batch_size;
    let prepared_records: Vec<_> = records
        .par_chunks(chunk_size)
        .flat_map(|chunk| {
            let texts: Vec<String> = chunk
                .iter()
                .map(|record| serde_json::to_string(record).unwrap())
                .collect();

            let embeddings = match
                generate_embeddings_batch(
                    &texts,
                    &args.embedding_model,
                    args.embedding_concurrency,
                    &args.embedding_url,
                    args.embedding_timeout
                )
            {
                Ok(embs) => embs,
                Err(e) => {
                    error!("Batch embedding failed: {}, falling back to single processing", e);
                    chunk
                        .par_iter()
                        .map(|record| {
                            generate_embedding(
                                &serde_json::to_string(record).unwrap(),
                                &args.embedding_model,
                                &args.embedding_url,
                                args.embedding_timeout
                            ).unwrap_or_default()
                        })
                        .collect()
                }
            };

            let _ = embedding_counter.fetch_add(chunk.len(), Ordering::Relaxed);

            chunk
                .iter()
                .zip(embeddings.into_iter())
                .map(|(record, vec)| {
                    let id = uuid::Uuid::new_v4().to_string();
                    let mut meta = record.clone();
                    meta.as_object_mut().unwrap().remove("table");
                    let table = record.get("table").unwrap().as_str().unwrap().to_string();
                    (table, id, vec, meta)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    *stop_animation.lock().unwrap() = true;
    animation_handle.join().unwrap();

    println!("\nEmbedding generation complete! Processing data...");

    let mut grouped_records: std::collections::HashMap<
        String,
        Vec<(String, Vec<f32>, serde_json::Value)>
    > = std::collections::HashMap::new();

    for (table, id, vec, meta) in prepared_records {
        grouped_records.entry(table).or_insert_with(Vec::new).push((id, vec, meta));
    }

    let processed_count = Arc::new(AtomicUsize::new(0));
    let total_to_process = total_records;
    let stop_storage_animation = Arc::new(Mutex::new(false));
    let storage_animation_stop = stop_storage_animation.clone();

    let storage_counter = processed_count.clone();
    let storage_animation = thread::spawn(move || {
        let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let mut spinner_idx = 0;

        while !*storage_animation_stop.lock().unwrap() {
            let count = storage_counter.load(Ordering::Relaxed);
            spinner_idx = (spinner_idx + 1) % spinner_chars.len();

            print!(
                "\r{} Storing in database... [{}/{}] ({}%)",
                spinner_chars[spinner_idx],
                count,
                total_to_process,
                (count * 100) / total_to_process.max(1)
            );
            stdout().flush().unwrap();

            thread::sleep(Duration::from_millis(80));
        }
        print!("\r{}\r", " ".repeat(80));
        stdout().flush().unwrap();
    });

    let max_bytes = args.batch_size_mb * 1024 * 1024;
    let db_counter = processed_count.clone();

    for (table, items) in grouped_records {
        for batch in items.chunks(10) {
            store_in_batches(&*database, &table, batch, max_bytes)?;
            db_counter.fetch_add(batch.len(), Ordering::Relaxed);
        }
    }

    *stop_storage_animation.lock().unwrap() = true;
    storage_animation.join().unwrap();

    let elapsed_time = start_time.elapsed();
    println!(
        "\nFinished processing {} records in {:.2} seconds ({:.1} records/sec)",
        total_records,
        elapsed_time.as_secs_f64(),
        (total_records as f64) / elapsed_time.as_secs_f64()
    );
    println!("Migration Complete.");
    Ok(())
}
