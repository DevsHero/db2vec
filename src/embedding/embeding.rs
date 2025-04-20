use log::{ debug, error, info, warn };
use reqwest::blocking::Client as HttpClient;
use serde_json::{ json, Value };
use std::{ time::Duration, sync::OnceLock };
use rayon::prelude::*;
use std::sync::atomic::{ AtomicUsize, Ordering };
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

pub fn generate_embedding(
    text: &str,
    model: &str,
    embedding_url: &str,
    timeout: u64
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
        max_tokens: 8000,
        timeout: Duration::from_secs(timeout),
        max_retries: 3,
        retry_delay: Duration::from_secs(2),
    };

    let client = HttpClient::builder().timeout(config.timeout).build()?;
    let service = EmbeddingService { client, config };
    service.generate_single(text)
}

pub fn generate_embeddings_batch(
    texts: &[String],
    model: &str,
    concurrency: usize,
    embedding_url: &str,
    timeout: u64
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
        max_tokens: 8000,
        timeout: Duration::from_secs(timeout),
        max_retries: 3,
        retry_delay: Duration::from_secs(2),
    };

    let client = HttpClient::builder().timeout(config.timeout).build()?;
    let service = EmbeddingService { client, config };

    service.generate_batch(texts)
}
