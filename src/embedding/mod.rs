pub mod embeding;

use crate::embedding::embeding::{ generate_embedding, generate_embeddings_batch };
use log::error;
use rayon::prelude::*;
use serde_json::Value;
use crate::cli::Args;
use std::sync::Arc;
use std::sync::atomic::{ AtomicUsize, Ordering };
use uuid::Uuid;
pub trait EmbeddingModel {
    fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>>;
}

pub struct EmbeddingService<T: EmbeddingModel> {
    model: T,
}

impl<T: EmbeddingModel> EmbeddingService<T> {
    pub fn new(model: T) -> Self {
        Self { model }
    }

    pub fn generate(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        self.model.generate_embedding(text)
    }
}
pub fn process_records_with_embeddings(
    records: Vec<Value>,
    args: &Args,
    embedding_counter: Arc<AtomicUsize>
) -> Vec<(String, String, Vec<f32>, Value)> {
    let chunk_size = args.embedding_batch_size;
    let prepared_records: Vec<_> = records
        .par_chunks(chunk_size)
        .flat_map(|chunk| {
            let texts: Vec<String> = chunk
                .iter()
                .map(|record| serde_json::to_string(record).unwrap())
                .collect();

            let embeddings = match generate_embeddings_batch(&texts) {
                Ok(embs) => embs,
                Err(e) => {
                    error!("Batch embedding failed: {}, falling back to single processing", e);
                    chunk
                        .par_iter()
                        .map(|record| {
                            generate_embedding(
                                &serde_json::to_string(record).unwrap()
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
                    let id = Uuid::new_v4().to_string();
                    let mut meta = record.clone();
                    meta.as_object_mut().unwrap().remove("table");
                    let table = record.get("table").unwrap().as_str().unwrap().to_string();
                    (table, id, vec, meta)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    prepared_records
}
