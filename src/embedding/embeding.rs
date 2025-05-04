use log::{ error, info };
use serde_json::Value;
use std::error::Error as StdError;
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::sync::Arc;
use tokio::runtime::Runtime;
use uuid::Uuid;
use crate::cli::Args;
use crate::embedding::{
    models::google::GoogleEmbeddingClient,
    models::ollama::OllamaEmbeddingClient, 
    models::tei::TeiEmbeddingClient,
    AsyncEmbeddingGenerator,
};

pub fn initialize_embedding_generator(
    args: &Args,
    override_url: Option<&str>,
) -> Result<Box<dyn AsyncEmbeddingGenerator + Send + Sync>, Box<dyn StdError + Sync + Send>> {
    let provider = args.embedding_provider.to_lowercase();
    info!("Selected embedding provider: {}", provider);

    let url = override_url
        .or_else(|| args.embedding_url.as_deref())
        .map(|s| s.to_string());

    match provider.as_str() {
        "tei" => {
     
            let url_to_use = match override_url {
                Some(url) => url,
                None => args.embedding_url.as_deref().unwrap_or("http://localhost:8080")
            };
            
            let client = TeiEmbeddingClient::new(
                url_to_use.to_string(),
                args.dimension,
                args.embedding_timeout 
            )?;
            Ok(Box::new(client))
        }

        "ollama" => {
            let ollama_url = url.unwrap_or_else(|| "http://localhost:11434".into());
            info!("🟢 Ollama client -> {}", ollama_url);
            let client = OllamaEmbeddingClient::new(
                &ollama_url,
                &args.embedding_model,
                args.dimension,
            )?;
            Ok(Box::new(client))
        }

        "google" => {
            let api_key = args.embedding_api_key
                .clone()
                .ok_or_else(|| "Missing EMBEDDING_API_KEY for Google".to_string())?;
            info!("🟢 Google client");
            let client = GoogleEmbeddingClient::new(
                api_key,
                Some(args.embedding_model.clone()),
                args.dimension,
            )?;
            Ok(Box::new(client))
        }

        other => Err(format!("Unsupported embedding provider: {}", other).into()),
    }
}

pub fn process_records_with_embeddings(
    records: Vec<Value>,
    args: &Args,
    embedding_counter: Arc<AtomicUsize>,
    generator: Box<dyn AsyncEmbeddingGenerator + Send + Sync>
) -> Result<Vec<(String, String, Vec<f32>, Value)>, Box<dyn StdError + Send + Sync>> {
    let chunk_size = args.embedding_batch_size;
    let total_records = records.len();
    let mut prepared_records = Vec::with_capacity(total_records);

    let rt = Runtime::new()?;

    for (chunk_idx, chunk) in records.chunks(chunk_size).enumerate() {
        info!(
            "Processing embedding chunk {}/{}",
            chunk_idx + 1,
            (total_records + chunk_size - 1) / chunk_size
        );

        let texts: Vec<String> = chunk
            .iter()
            .map(|record| {
                record
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .filter(|(k, _)| *k != "table" && *k != "id")
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| record.to_string())
            })
            .collect();

        let embeddings_result = rt.block_on(generator.generate_embeddings_batch(&texts));

        match embeddings_result {
            Ok(embeddings) => {
                if embeddings.len() != chunk.len() {
                    error!(
                        "CRITICAL: Embedding generator returned {} results for {} inputs in chunk {}",
                        embeddings.len(),
                        chunk.len(),
                        chunk_idx + 1
                    );
                    return Err(
                        format!(
                            "Embedding generator returned incomplete results: got {}/{}",
                            embeddings.len(),
                            chunk.len()
                        ).into()
                    );
                }

                let chunk_results: Vec<_> = chunk
                    .iter()
                    .zip(embeddings.into_iter())
                    .map(|(record, vec)| {
                        let id = Uuid::new_v4().to_string();
                        let mut meta = record.clone();
                        let table = meta
                            .get("table")
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown_table")
                            .to_string();
                        if let Some(_obj) = meta.as_object_mut() {
                        }
                        (table, id, vec, meta)
                    })
                    .collect();

                prepared_records.extend(chunk_results);
                embedding_counter.fetch_add(chunk.len(), Ordering::Relaxed);
            }
            Err(e) => {
                error!("CRITICAL: Embedding generation failed for chunk {}: {}", chunk_idx + 1, e);
                return Err(format!("Embedding generation failed: {}", e).into());
            }
        }
    }

    Ok(prepared_records)
}