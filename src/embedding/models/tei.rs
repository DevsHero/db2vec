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
    binary_path: String,
    dimension: usize,
}

impl TeiEmbeddingClient {
    pub fn new(
        binary_path: String,
        dimension: usize, 
        timeout_secs: u64
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let mut corrected_url = binary_path;
        if !corrected_url.ends_with("/embed") {
            warn!("TEI server URL '{}' does not end with /embed. Appending it.", corrected_url);
            corrected_url = format!("{}/embed", corrected_url.trim_end_matches('/'));
        }

        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(timeout_secs)).build()?,
            binary_path: corrected_url,
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
            self.binary_path
        );

        let request_payload = TeiRequest {
            inputs: texts.to_vec(),
            truncate: None, 
        };

        let response = self.client.post(&self.binary_path).json(&request_payload).send().await;

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    let embeddings = res.json::<TeiResponse>().await?;
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
                    Ok(embeddings)
                } else {
                    let status = res.status();
                    let error_text = res
                        .text().await
                        .unwrap_or_else(|_| "Failed to read error body".to_string());
                    error!("TEI server returned error {}: {}", status, error_text);
                    Err(format!("TEI server error {}: {}", status, error_text).into())
                }
            }
            Err(e) => {
                error!("Failed to send request to TEI server: {}", e);
                Err(Box::new(e))
            }
        }
    }

    fn get_dimension(&self) -> usize {
     self.dimension
    }
}
