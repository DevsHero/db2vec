use log::{ debug, error, info, warn };
use reqwest::blocking::Client as HttpClient;
use serde_json::{ json, Value };
use std::{ time::Duration, sync::OnceLock };
use rayon::prelude::*;
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::sync::Arc;

use crate::cli::Args;

pub struct EmbeddingConfig {
    pub model: String,
    pub url: String,
    pub batch_size: usize,
    pub max_concurrency: usize,
    pub max_tokens: usize,
    pub timeout: Duration,
    pub max_retries: usize,
    pub retry_delay: Duration,
}

static SERVICE: OnceLock<EmbeddingService> = OnceLock::new();
static ACTIVE_REQUESTS: AtomicUsize = AtomicUsize::new(0);

/// Initialize the global embedding service with configuration
pub fn initialize_from_args(args: &Args) {
    initialize(
        &args.embedding_model,
        &args.embedding_url,
        args.embedding_batch_size,
        args.embedding_concurrency,
        args.embedding_max_tokens,
        args.embedding_timeout
    );
}

pub fn initialize(
    model: &str,
    url: &str,
    batch_size: usize,
    concurrency: usize,
    max_tokens: usize,
    timeout: u64
) {
    let api_url = if url.ends_with("/api/embeddings") {
        url.to_string()
    } else {
        format!("{}/api/embeddings", url.trim_end_matches('/'))
    };

    let api_url_for_log = api_url.clone();
    let config = EmbeddingConfig {
        model: model.to_string(),
        url: api_url,
        batch_size,
        max_concurrency: concurrency,
        max_tokens,
        timeout: Duration::from_secs(timeout),
        max_retries: 3,
        retry_delay: Duration::from_secs(2),
    };

    let client = HttpClient::builder()
        .timeout(config.timeout)
        .build()
        .expect("Failed to create HTTP client");

    let service = EmbeddingService { client, config };

    if SERVICE.set(service).is_ok() {
        info!("Initialized embedding service with model '{}' at {}", model, api_url_for_log);
    }
}

struct EmbeddingService {
    client: HttpClient,
    config: EmbeddingConfig,
}

impl EmbeddingService {
    fn generate_single(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let current = ACTIVE_REQUESTS.fetch_add(1, Ordering::SeqCst);
        if current >= self.config.max_concurrency {
            debug!("Waiting for available embedding slot (current: {})", current);
            std::thread::sleep(Duration::from_millis(100));
        }

        struct RequestGuard;
        impl Drop for RequestGuard {
            fn drop(&mut self) {
                ACTIVE_REQUESTS.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let _guard = RequestGuard;
        let trimmed_text = if text.len() > self.config.max_tokens {
            &text[0..self.config.max_tokens]
        } else {
            text
        };

        for attempt in 0..self.config.max_retries {
            if attempt > 0 {
                debug!(
                    "Retrying embedding request (attempt {}/{})",
                    attempt + 1,
                    self.config.max_retries
                );
                std::thread::sleep(self.config.retry_delay);
            }

            let response = self.client
                .post(&self.config.url)
                .header("Content-Type", "application/json")
                .json(
                    &json!({
                        "model": self.config.model,
                        "prompt": trimmed_text
                    })
                )
                .send()?;

            if !response.status().is_success() {
                warn!("Embedding API returned status: {}", response.status());
                continue;
            }

            let json = response.json::<Value>()?;
            if let Some(embedding_array) = json["embedding"].as_array() {
                let embedding: Vec<f32> = embedding_array
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();

                debug!("Generated embedding with {} dimensions", embedding.len());
                return Ok(embedding);
            } else {
                error!("Unexpected response structure: {:?}", json);
            }
        }

        Err("Failed to get embeddings after retries".into())
    }

    fn generate_batch(
        &self,
        texts: &[String]
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        if texts.len() > self.config.batch_size {
            info!(
                "Large batch of {} texts split into chunks of {}",
                texts.len(),
                self.config.batch_size
            );

            let mut results = Vec::new();
            for (i, chunk) in texts.chunks(self.config.batch_size).enumerate() {
                info!(
                    "Processing chunk {}/{}",
                    i + 1,
                    (texts.len() + self.config.batch_size - 1) / self.config.batch_size
                );

                let chunk_results = self.process_batch_chunk(chunk)?;
                results.extend(chunk_results);
            }

            return Ok(results);
        }

        self.process_batch_chunk(texts)
    }

    fn process_batch_chunk(
        &self,
        texts: &[String]
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        info!("Batch embedding {} texts using model {}", texts.len(), self.config.model);

        let response = self.client
            .post(&self.config.url)
            .json(
                &json!({
                    "model": self.config.model,
                    "prompts": texts
                })
            )
            .send();

        if let Ok(resp) = response {
            if resp.status().is_success() {
                let parsed: Value = resp.json()?;

                if let Some(embeddings) = parsed.get("embeddings").and_then(|e| e.as_array()) {
                    let mut result = Vec::with_capacity(embeddings.len());

                    for emb in embeddings {
                        if let Some(vector) = emb.get("embedding").and_then(|v| v.as_array()) {
                            let embedding: Vec<f32> = vector
                                .iter()
                                .filter_map(|v| v.as_f64().map(|f| f as f32))
                                .collect();

                            result.push(embedding);
                        }
                    }

                    if result.len() == texts.len() {
                        info!("Successfully processed batch of {} embeddings", result.len());
                        return Ok(result);
                    }
                }

                warn!(
                    "Batch embedding response format unexpected, falling back to individual requests"
                );
            }
        }

        info!("Using fallback method: parallel individual embedding requests");
        let results: Vec<Vec<f32>> = texts
            .par_iter()
            .map(|text| {
                self.generate_single(text).unwrap_or_else(|e| {
                    error!("Single embedding failed: {}", e);
                    Vec::new()
                })
            })
            .collect();

        let valid_results: Vec<Vec<f32>> = results
            .into_iter()
            .filter(|v| !v.is_empty())
            .collect();

        if valid_results.len() != texts.len() {
            warn!(
                "Some embeddings failed: got {} out of {} requested",
                valid_results.len(),
                texts.len()
            );
        }

        Ok(valid_results)
    }
}

/// Generate embedding for a single text using the global service
pub fn generate_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if let Some(service) = SERVICE.get() {
        service.generate_single(text)
    } else {
        Err("Embedding service not initialized".into())
    }
}

/// Generate embeddings for multiple texts using the global service
pub fn generate_embeddings_batch(
    texts: &[String]
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    if texts.is_empty() {
        return Ok(vec![]);
    }

    if let Some(service) = SERVICE.get() {
        service.generate_batch(texts)
    } else {
        Err("Embedding service not initialized".into())
    }
}

/// Process records and generate embeddings
pub fn process_records_with_embeddings(
    records: Vec<Value>,
    embedding_counter: Arc<AtomicUsize>
) -> Vec<(String, String, Vec<f32>, Value)> {
    let chunk_size = SERVICE.get()
        .map(|service| service.config.batch_size)
        .unwrap_or(16); // Default if not initialized

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

            // Update the embedding counter
            embedding_counter.fetch_add(chunk.len(), Ordering::Relaxed);

            // Process and return results
            chunk
                .iter()
                .zip(embeddings.into_iter())
                .map(|(record, vec)| {
                    let id = uuid::Uuid::new_v4().to_string();
                    let mut meta = record.clone();
                    let table = record
                        .get("table")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown_table")
                        .to_string();

                    if let Some(obj) = meta.as_object_mut() {
                        obj.remove("table");
                    }

                    (table, id, vec, meta)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    prepared_records
}

// Keep the original functions for backward compatibility but mark as deprecated
#[deprecated(note = "Use initialize_from_args instead")]
pub fn generate_embedding_with_params(
    text: &str,
    model: &str,
    embedding_url: &str,
    timeout: u64,
    max_tokens: usize
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if let Some(service) = SERVICE.get() {
        return service.generate_single(text);
    }

    let api_url = if embedding_url.ends_with("/api/embeddings") {
        embedding_url.to_string()
    } else {
        format!("{}/api/embeddings", embedding_url.trim_end_matches('/'))
    };

    let config = EmbeddingConfig {
        model: model.to_string(),
        url: api_url,
        batch_size: 16,
        max_concurrency: 4,
        max_tokens,
        timeout: Duration::from_secs(timeout),
        max_retries: 3,
        retry_delay: Duration::from_secs(2),
    };

    let client = HttpClient::builder().timeout(config.timeout).build()?;
    let service = EmbeddingService { client, config };
    service.generate_single(text)
}

#[deprecated(note = "Use initialize_from_args instead")]
pub fn generate_embeddings_batch_with_params(
    texts: &[String],
    model: &str,
    concurrency: usize,
    embedding_url: &str,
    timeout: u64,
    max_tokens: usize
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    if texts.is_empty() {
        return Ok(vec![]);
    }

    let api_url = if embedding_url.ends_with("/api/embeddings") {
        embedding_url.to_string()
    } else {
        format!("{}/api/embeddings", embedding_url.trim_end_matches('/'))
    };

    let config = EmbeddingConfig {
        model: model.to_string(),
        url: api_url,
        batch_size: 16,
        max_concurrency: concurrency,
        max_tokens,
        timeout: Duration::from_secs(timeout),
        max_retries: 3,
        retry_delay: Duration::from_secs(2),
    };

    let client = HttpClient::builder().timeout(config.timeout).build()?;
    let service = EmbeddingService { client, config };
    service.generate_batch(texts)
}
