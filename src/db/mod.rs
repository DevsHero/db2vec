pub mod redis;
pub mod qdrant;
pub mod chroma;
pub mod milvus;
pub mod surreal;
pub use redis::RedisDatabase;
pub use milvus::MilvusDatabase;
pub use qdrant::QdrantDatabase;
pub use chroma::ChromaDatabase;
pub use surreal::SurrealDatabase;
use serde_json::Value;
use std::error::Error;
pub trait Database {
    fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> where Self: Sized;

    fn store_vector(
        &self,
        table: &str,
        key: &str,
        vector: &[f32],
        data: &Value
    ) -> Result<(), Box<dyn Error>>;
}
