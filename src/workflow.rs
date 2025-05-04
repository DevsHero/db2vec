use crate::cli::Args;
use crate::db::{ Database, DbError, store_in_batches };
use crate::embedding::embeding::{ initialize_embedding_generator, process_records_with_embeddings };
use crate::util::spinner::start_spinner_animation;
use crate::util::handle_tei::{start_and_wait_for_tei, ManagedProcess};
use log::{ info, warn, error };
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::time::Instant;

pub struct MigrationStats {
    pub total_records: usize,
    pub processed_records: usize,
    pub elapsed_seconds: f64,
}

pub fn execute_migration_workflow(
    records: Vec<Value>,
    database: &dyn Database,
    args: &Args,
) -> Result<MigrationStats, DbError> {
    let total_records = records.len();
    if total_records == 0 {
        warn!("No records to process");
        return Ok(MigrationStats {
            total_records: 0,
            processed_records: 0,
            elapsed_seconds: 0.0,
        });
    }

    let mut tei_process: Option<ManagedProcess> = None;
    let mut override_url: Option<String> = None;

    if args.embedding_provider == "tei" && args.embedding_url.is_none() {
        let args = args.clone();
        let (proc, url) = std::thread::spawn(move || start_and_wait_for_tei(&args))
            .join()
            .map_err(|e| format!("TEI thread panicked: {:?}", e))??;
        tei_process = Some(proc);
        override_url = Some(url);
    }

    let generator = initialize_embedding_generator(args, override_url.as_deref())
        .map_err(|e| DbError::from(format!("Init embed gen failed: {}", e)))?;

    let start_time = Instant::now();
    let embedding_count = Arc::new(AtomicUsize::new(0));
    let embedding_animation = start_spinner_animation(
        embedding_count.clone(),
        total_records,
        "Generating embeddings"
    );

    info!("Starting embedding generation for {} records", total_records);

    let prepared_records = match
        process_records_with_embeddings(records, args, embedding_count.clone(), generator)
    {
        Ok(records) => records,
        Err(e) => {
            embedding_animation.stop();
            error!("CRITICAL: Embedding generation failed: {}", e);
            return Err(format!("Embedding generation critical error: {}", e).into());
        }
    };

    embedding_animation.stop();

    if prepared_records.is_empty() {
        warn!("No records were prepared for storage after embedding process.");
    } else {
        println!("\nEmbedding generation complete! Storing data...");

        let mut grouped_records: HashMap<String, Vec<(String, Vec<f32>, Value)>> = HashMap::new();
        for (table, id, vec, meta) in prepared_records {
            grouped_records.entry(table).or_insert_with(Vec::new).push((id, vec, meta));
        }

        let processed_count = Arc::new(AtomicUsize::new(0));
        let storage_animation = start_spinner_animation(
            processed_count.clone(),
            total_records,
            "Storing in database"
        );

        let max_payload_bytes = args.max_payload_size_mb * 1024 * 1024;
        let chunk_size = args.chunk_size;

        for (table, items) in grouped_records {
            info!("Storing {} items for table '{}'", items.len(), table);
            for batch in items.chunks(chunk_size) {
                match store_in_batches(database, &table, batch, max_payload_bytes) {
                    Ok(_) => {
                        let _ = processed_count.fetch_add(batch.len(), Ordering::Relaxed);
                    }
                    Err(e) => {
                        storage_animation.stop();
                        error!("CRITICAL: Database storage error for table '{}': {}", table, e);
                        return Err(format!("Database storage error: {}", e).into());
                    }
                }
            }
        }
        storage_animation.stop();
    }

    let elapsed_time = start_time.elapsed();
    let final_count = embedding_count.load(Ordering::Relaxed);

    println!(
        "\nFinished processing {} records in {:.2} seconds ({:.1} records/sec)",
        final_count,
        elapsed_time.as_secs_f64(),
        if elapsed_time.as_secs_f64() > 0.0 {
            (final_count as f64) / elapsed_time.as_secs_f64()
        } else {
            0.0
        }
    );
    println!("Migration Complete.");

    if let Some(mut p) = tei_process {
        let _ = p.kill();
    }

    Ok(MigrationStats {
        total_records,
        processed_records: final_count,
        elapsed_seconds: elapsed_time.as_secs_f64(),
    })
}