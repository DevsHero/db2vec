use crate::embedding::AsyncEmbeddingGenerator;
use async_trait::async_trait;
use log::{ info, error, warn };
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder,
    SentenceEmbeddingsModel,
    SentenceEmbeddingsModelType,
};
use std::error::Error as StdError;
use std::path::PathBuf;
use std::sync::{ Arc, Mutex };
use tokio::task;
type SafeSentenceEmbeddingsModel = Arc<Mutex<SentenceEmbeddingsModel>>;

pub struct RustBertEmbeddingClient {
    model: SafeSentenceEmbeddingsModel,
    dimension: usize,
    model_identifier: String,
}

impl RustBertEmbeddingClient {
    pub fn new(
        model_identifier_or_path: &str,
        device: tch::Device
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let model_name_to_log = model_identifier_or_path;
        let dimension = 384;

        info!(
            "Initializing RustBert Embedding Client with hardcoded model: all-MiniLM-L6-v2 (Dimension: {}) on device: {:?}",
            dimension,
            device
        );
        info!("Provided identifier/path '{}' will be used for logging.", model_name_to_log);

        let model_result = if PathBuf::from(model_identifier_or_path).exists() {
            warn!("Loading local model from path: {}", model_identifier_or_path);
            SentenceEmbeddingsBuilder::local(model_identifier_or_path)
                .with_device(device)
                .create_model()
        } else {
            info!("Loading remote model: all-MiniLM-L6-v2");
            SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
                .with_device(device)
                .create_model()
        };

        let model = match model_result {
            Ok(m) => m,
            Err(e) => {
                error!(
                    "Failed to load RustBert model 'all-MiniLM-L6-v2' (or path '{}'): {}",
                    model_identifier_or_path,
                    e
                );
                return Err(format!("Failed to load RustBert model: {}", e).into());
            }
        };
        let safe_model = Arc::new(Mutex::new(model));
        Ok(Self {
            model: safe_model,
            dimension,
            model_identifier: model_name_to_log.to_string(),
        })
    }
}

#[async_trait]
impl AsyncEmbeddingGenerator for RustBertEmbeddingClient {
    async fn generate_embeddings_batch(
        &self,
        texts: &[String]
    ) -> Result<Vec<Vec<f32>>, Box<dyn StdError + Send + Sync>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        info!(
            "RustBert: Generating embeddings for {} texts using model identifier '{}' (Actual: all-MiniLM-L6-v2)",
            texts.len(),
            self.model_identifier
        );
        let texts_clone = texts.to_vec();
        let model_arc_clone = self.model.clone();
        let expected_dimension = self.dimension;
        let result = task::spawn_blocking(move || {
            let model_guard = match model_arc_clone.lock() {
                Ok(guard) => guard,
                Err(_poisoned) => {
                    error!("RustBert model mutex is poisoned!");
                    return Err("RustBert model mutex is poisoned!".into());
                }
            };

            let embeddings_result = model_guard.encode(&texts_clone);

            embeddings_result.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)
        }).await;

        match result {
            Ok(Ok(embeddings)) => {
                for (i, emb) in embeddings.iter().enumerate() {
                    if emb.len() != expected_dimension {
                        error!(
                            "RustBert embedding dimension mismatch for text {}: expected {}, got {}",
                            i,
                            expected_dimension,
                            emb.len()
                        );
                        return Err(
                            format!(
                                "Dimension mismatch: expected {}, got {}",
                                expected_dimension,
                                emb.len()
                            ).into()
                        );
                    }
                }
                info!("RustBert: Successfully generated {} embeddings", embeddings.len());
                Ok(embeddings)
            }
            Ok(Err(e)) => {
                error!("RustBert embedding generation failed inside blocking task: {}", e);
                Err(e)
            }
            Err(e) => {
                error!("RustBert embedding task failed: {}", e);
                Err(Box::new(e) as Box<dyn StdError + Send + Sync>)
            }
        }
    }

    fn get_dimension(&self) -> usize {
        self.dimension
    }
}
