use crate::embedding::AsyncEmbeddingGenerator;
use async_trait::async_trait;
use log::{ info, error, warn, debug };
use reqwest::Client;
use serde_json::{ json, Value };
use std::error::Error as StdError;
use std::time::Duration;
use tokio::time::sleep;

pub struct GoogleEmbeddingClient {
    client: Client,
    api_key: String,
    model_name: String,
    dimension: usize,
    task_type: String,
    request_delay_ms: u64,
}

impl GoogleEmbeddingClient {
    pub fn new(
        api_key: String,
        model: Option<String>,
        dimension: usize
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let embed_model = model.unwrap_or_else(|| "text-embedding-004".to_string());
        let clean_model = embed_model.trim_start_matches("models/").to_string();
        let default_delay_ms = 1100;

        info!(
            "Initializing Google Embedding Client with model: {}, dimension: {}, request delay: {}ms",
            clean_model,
            dimension,
            default_delay_ms
        );

        Ok(Self {
            client: Client::new(),
            api_key,
            model_name: clean_model,
            dimension,
            task_type: "SEMANTIC_SIMILARITY".to_string(),
            request_delay_ms: default_delay_ms,
        })
    }

    pub fn with_task_type(mut self, task_type: &str) -> Self {
        self.task_type = task_type.to_string();
        self
    }

    pub fn with_request_delay(mut self, delay_ms: u64) -> Self {
        self.request_delay_ms = delay_ms;
        info!("Set Google request delay to {}ms", delay_ms);
        self
    }
}

#[async_trait]
impl AsyncEmbeddingGenerator for GoogleEmbeddingClient {
    async fn generate_embeddings_batch(
        &self,
        texts: &[String]
    ) -> Result<Vec<Vec<f32>>, Box<dyn StdError + Send + Sync>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        info!(
            "Google: Generating embeddings for {} texts using model {} with task type {} (delay: {}ms)",
            texts.len(),
            self.model_name,
            self.task_type,
            self.request_delay_ms
        );

        let mut results = Vec::with_capacity(texts.len());

        for (i, text) in texts.iter().enumerate() {
            if i > 0 {
                sleep(Duration::from_millis(self.request_delay_ms)).await;
            }

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
                self.model_name,
                self.api_key
            );

            let request_body =
                json!({
                "model": format!("models/{}", self.model_name),
                "content": {
                    "parts": [
                        {
                            "text": text
                        }
                    ]
                },
                "taskType": self.task_type
            });

            debug!("Request URL: {}", url);
            debug!("Request body: {}", request_body.to_string());

            let response = self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send().await;

            match response {
                Ok(res) => {
                    let status = res.status();

                    if status.is_success() {
                        match res.json::<Value>().await {
                            Ok(json_response) => {
                                debug!("Success Response: {:?}", json_response);

                                if
                                    let Some(values) = json_response
                                        .get("embedding")
                                        .and_then(|e| e.get("values"))
                                        .and_then(|v| v.as_array())
                                {
                                    let embedding: Vec<f32> = values
                                        .iter()
                                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                                        .collect();

                                    results.push(embedding);
                                } else {
                                    error!(
                                        "CRITICAL: Invalid response format: {:?}",
                                        json_response
                                    );
                                    return Err("Invalid embedding response format".into());
                                }
                            }
                            Err(e) => {
                                error!("CRITICAL: Failed to parse response JSON: {}", e);
                                return Err(format!("JSON parsing error: {}", e).into());
                            }
                        }
                    } else {
                        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                            warn!(
                                "Rate limit hit (429). Consider increasing delay or checking quota."
                            );
                        }

                        let error_text = match res.text().await {
                            Ok(text) => text,
                            Err(_) => "Failed to read error response".to_string(),
                        };
                        error!("CRITICAL: Google API error ({}): {}", status, error_text);

                        if let Ok(error_json) = serde_json::from_str::<Value>(&error_text) {
                            error!("Error details: {:?}", error_json);
                            if
                                let Some(message) = error_json
                                    .get("error")
                                    .and_then(|e| e.get("message"))
                                    .and_then(|m| m.as_str())
                            {
                                error!("Error message: {}", message);
                                return Err(format!("Google API error: {}", message).into());
                            }
                        }

                        return Err(format!("Google API error ({}): {}", status, error_text).into());
                    }
                }
                Err(e) => {
                    error!("CRITICAL: Request failed: {}", e);
                    return Err(format!("Network error: {}", e).into());
                }
            }
        }

        info!("Google: Successfully generated {} embeddings", results.len());
        Ok(results)
    }

    fn get_dimension(&self) -> usize {
        self.dimension
    }
}
