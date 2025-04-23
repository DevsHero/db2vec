pub mod embeding;

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
