pub mod embeding;
pub mod models;

use async_trait::async_trait;
use std::error::Error as StdError;

#[async_trait]
pub trait AsyncEmbeddingGenerator: Send + Sync {
    async fn generate_embeddings_batch(
        &self,
        texts: &[String]
    ) -> Result<Vec<Vec<f32>>, Box<dyn StdError + Send + Sync>>;

    fn get_dimension(&self) -> usize;
}

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
