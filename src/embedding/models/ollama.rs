use crate::embedding::AsyncEmbeddingGenerator;
use async_trait::async_trait;
use log::{ error, info, warn };
use reqwest::Client as AsyncHttpClient;
use serde_json::{ json, Value };
use std::{ error::Error as StdError, time::Duration };
use futures::future::join_all;

pub struct OllamaEmbeddingClient {
    client: AsyncHttpClient,
    api_url: String,
    model: String,
    dimension: usize,
}

impl OllamaEmbeddingClient {
    pub fn new(
        base_url: &str,
        model: &str,
        dimension: usize
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let api_url = if base_url.ends_with("/api/embeddings") {
            base_url.to_string()
        } else {
            format!("{}/api/embeddings", base_url.trim_end_matches('/'))
        };

        let client = AsyncHttpClient::builder().timeout(Duration::from_secs(20)).build()?;

        Ok(Self {
            client,
            api_url,
            model: model.to_string(),
            dimension,
        })
    }

    async fn generate_single_embedding(
        &self,
        text: &str
    ) -> Result<Vec<f32>, Box<dyn StdError + Send + Sync>> {
        let response = self.client
            .post(&self.api_url)
            .header("Content-Type", "application/json")
            .json(&json!({ "model": &self.model, "prompt": text }))
            .send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text().await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            warn!("Ollama API (single) returned status: {}, body: {}", status, error_body);
            return Err(format!("Ollama API error: {}", status).into());
        }

        let json_body = response.json::<Value>().await?;
        if let Some(embedding_array) = json_body["embedding"].as_array() {
            let embedding: Vec<f32> = embedding_array
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            if embedding.len() != self.dimension {
                warn!(
                    "Ollama returned embedding with dimension {}, expected {}",
                    embedding.len(),
                    self.dimension
                );

                return Err(
                    format!(
                        "Dimension mismatch: expected {}, got {}",
                        self.dimension,
                        embedding.len()
                    ).into()
                );
            }
            Ok(embedding)
        } else {
            Err("Unexpected response structure from Ollama API".into())
        }
    }
}

#[async_trait]
impl AsyncEmbeddingGenerator for OllamaEmbeddingClient {
    async fn generate_embeddings_batch(
        &self,
        texts: &[String]
    ) -> Result<Vec<Vec<f32>>, Box<dyn StdError + Send + Sync>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        info!("Ollama: Generating embeddings for {} texts", texts.len());

        let response = self.client
            .post(&self.api_url)
            .json(&json!({ "model": &self.model, "prompts": texts }))
            .send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<Value>().await {
                    Ok(parsed) => {
                        if
                            let Some(embeddings) = parsed
                                .get("embeddings")
                                .and_then(|e| e.as_array())
                        {
                            let mut result = Vec::with_capacity(embeddings.len());
                            let mut success_count = 0;
                            for (i, emb_val) in embeddings.iter().enumerate() {
                                if
                                    let Some(vector) = emb_val
                                        .get("embedding")
                                        .and_then(|v| v.as_array())
                                {
                                    let embedding: Vec<f32> = vector
                                        .iter()
                                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                                        .collect();

                                    if embedding.len() == self.dimension {
                                        result.push(embedding);
                                        success_count += 1;
                                    } else {
                                        warn!(
                                            "Ollama batch item {} dimension mismatch: expected {}, got {}",
                                            i,
                                            self.dimension,
                                            embedding.len()
                                        );
                                        result.push(vec![0.0; self.dimension]);
                                    }
                                } else {
                                    warn!("Ollama batch item {} missing 'embedding' array", i);
                                    result.push(vec![0.0; self.dimension]);
                                }
                            }

                            if result.len() == texts.len() {
                                info!(
                                    "Ollama: Successfully processed batch of {} embeddings ({} succeeded)",
                                    result.len(),
                                    success_count
                                );
                                return Ok(result);
                            } else {
                                warn!(
                                    "Ollama batch result count mismatch: expected {}, got {}",
                                    texts.len(),
                                    result.len()
                                );
                            }
                        } else {
                            warn!("Ollama batch response missing 'embeddings' array");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse Ollama batch response: {}. Falling back.", e);
                    }
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let error_body = resp
                    .text().await
                    .unwrap_or_else(|_| "Failed to read error body".to_string());
                warn!(
                    "Ollama batch API returned status: {}. Body: {}. Falling back.",
                    status,
                    error_body
                );
            }
            Err(e) => {
                warn!("Ollama batch request failed: {}. Falling back.", e);
            }
        }

        info!(
            "Ollama: Using fallback: parallel individual embedding requests for {} texts",
            texts.len()
        );
        let futures: Vec<_> = texts
            .iter()
            .map(|text| self.generate_single_embedding(text))
            .collect();

        let results: Vec<Result<Vec<f32>, _>> = join_all(futures).await;

        let final_embeddings: Vec<Vec<f32>> = results
            .into_iter()
            .map(|res| {
                match res {
                    Ok(embedding) => embedding,
                    Err(e) => {
                        error!("Ollama single embedding failed during fallback: {}", e);
                        vec![0.0; self.dimension]
                    }
                }
            })
            .collect();

        Ok(final_embeddings)
    }

    fn get_dimension(&self) -> usize {
        self.dimension
    }
}
