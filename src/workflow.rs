use crate::cli::Args;
use crate::db::{ Database, DbError, store_in_batches };
use crate::embedding::process_records_with_embeddings;
use crate::util::spinner::start_spinner_animation;
use log::info;
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
    args: &Args
) -> Result<MigrationStats, DbError> {
    let total_records = records.len();
    let start_time = Instant::now();
    let embedding_count = Arc::new(AtomicUsize::new(0));
    let embedding_animation = start_spinner_animation(
        embedding_count.clone(),
        total_records,
        "Generating embeddings"
    );
    let prepared_records = process_records_with_embeddings(records, args, embedding_count.clone());

    embedding_animation.stop();
    println!("\nEmbedding generation complete! Processing data...");

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
            store_in_batches(database, &table, batch, max_payload_bytes)?;
            processed_count.fetch_add(batch.len(), Ordering::Relaxed);
        }
    }

    storage_animation.stop();

    let elapsed_time = start_time.elapsed();
    let final_count = processed_count.load(Ordering::Relaxed);

    println!(
        "\nFinished processing {} records in {:.2} seconds ({:.1} records/sec)",
        total_records,
        elapsed_time.as_secs_f64(),
        (total_records as f64) / elapsed_time.as_secs_f64()
    );
    println!("Migration Complete.");

    Ok(MigrationStats {
        total_records,
        processed_records: final_count,
        elapsed_seconds: elapsed_time.as_secs_f64(),
    })
}
