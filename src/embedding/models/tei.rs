use crate::embedding::AsyncEmbeddingGenerator;
use async_trait::async_trait;
use log::{ info, error, warn };
use reqwest::Client;
use serde::Serialize;
use std::error::Error as StdError;
use std::time::Duration;

#[derive(Serialize)]
struct TeiRequest {
    inputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncate: Option<bool>, 
}

type TeiResponse = Vec<Vec<f32>>;

pub struct TeiEmbeddingClient {
    client: Client,
    api_url: String,  
    dimension: usize,
}

impl TeiEmbeddingClient {
    pub fn new(
        api_url: String, 
        dimension: usize, 
        timeout_secs: u64
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let api_endpoint = if !api_url.ends_with("/embed") {
            format!("{}/embed", api_url.trim_end_matches('/'))
        } else {
            api_url
        };
        
        warn!("TEI server URL: {}", api_endpoint); 
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(timeout_secs)).build()?,
            api_url: api_endpoint, 
            dimension,
        })
    }
}

#[async_trait]
impl AsyncEmbeddingGenerator for TeiEmbeddingClient {
    async fn generate_embeddings_batch(
        &self,
        texts: &[String]
    ) -> Result<Vec<Vec<f32>>, Box<dyn StdError + Send + Sync>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        info!(
            "TEI Client: Generating embeddings for {} texts via {}",
            texts.len(),
            self.api_url
        );

        let request_payload = TeiRequest {
            inputs: texts.to_vec(),
            truncate: None, 
        };

    
        let mut retries = 3;
        let mut last_error = None;
        
        while retries > 0 {
            match self.client.post(&self.api_url).json(&request_payload).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let embeddings = response.json::<TeiResponse>().await?;
                        if embeddings.len() != texts.len() {
                            error!(
                                "TEI Client: Mismatch in response length. Expected {}, got {}.",
                                texts.len(),
                                embeddings.len()
                            );
                            return Err(
                                format!(
                                    "TEI response length mismatch: expected {}, got {}",
                                    texts.len(),
                                    embeddings.len()
                                ).into()
                            );
                        }
                        for emb in &embeddings {
                            if emb.len() != self.dimension {
                                error!(
                                    "TEI Client: Mismatch in embedding dimension. Expected {}, got {}.",
                                    self.dimension,
                                    emb.len()
                                );
                                return Err(
                                    format!(
                                        "TEI dimension mismatch: expected {}, got {}",
                                        self.dimension,
                                        emb.len()
                                    ).into()
                                );
                            }
                        }
                        info!("TEI Client: Successfully generated {} embeddings", embeddings.len());
                        return Ok(embeddings);
                    } else {
                        let status = response.status();
                        let error_text = response
                            .text().await
                            .unwrap_or_else(|_| "Failed to read error body".to_string());
                        error!("TEI server returned error {}: {}", status, error_text);
                        return Err(format!("TEI server error {}: {}", status, error_text).into());
                    }
                },
                Err(e) => {
                    warn!("TEI request failed (retries left: {}): {}", retries - 1, e);
                    retries -= 1;
                    last_error = Some(e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }
            }
        }

        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed after multiple retries: {}", last_error.unwrap())
        )))
    }

    fn get_dimension(&self) -> usize {
        self.dimension
    }
}
