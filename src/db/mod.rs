pub mod redis;
pub mod qdrant;
pub mod chroma;
pub mod milvus;
pub mod surreal;
pub mod pinecone;
pub use redis::RedisDatabase;
pub use milvus::MilvusDatabase;
pub use qdrant::QdrantDatabase;
pub use chroma::ChromaDatabase;
pub use surreal::SurrealDatabase;
pub use pinecone::PineconeDatabase;
use serde_json::Value;
use std::error::Error;
use crate::cli::Args;

pub type DbError = Box<dyn Error + Send + Sync>;

pub trait Database: Send + Sync {
    
    fn store_vector(&self, table: &str, items: &[(String, Vec<f32>, Value)]) -> Result<(), DbError>;
}

pub fn select_database(args: &Args) -> Result<Box<dyn Database>, DbError> {
    let database: Box<dyn Database> = match args.vector_export_type.as_str() {
        "redis" => Box::new(RedisDatabase::new(args)?),
        "qdrant" => Box::new(QdrantDatabase::new(args)?),
        "chroma" => Box::new(ChromaDatabase::new(args)?),
        "milvus" => Box::new(MilvusDatabase::new(args)?),
        "surreal" => Box::new(SurrealDatabase::new(args)?),
        "pinecone" => Box::new(PineconeDatabase::new(args)?),
        _ => {
            return Err("Unsupported database type".into());
        }
    };

    Ok(database)
}

pub fn store_in_batches(
    db: &dyn Database,
    table: &str,
    items: &[(String, Vec<f32>, Value)],
    max_bytes: usize
) -> Result<(), DbError> {
    let mut start = 0;
    let mut cur_size = 0;
    for (i, (id, vec, meta)) in items.iter().enumerate() {
        let meta_json = serde_json::to_string(meta)?;
        let rec_size = id.len() + vec.len() * 4 + meta_json.len();
        if cur_size + rec_size > max_bytes && start < i {
            db.store_vector(table, &items[start..i])?;
            start = i;
            cur_size = rec_size;
        } else {
            cur_size += rec_size;
        }
    }
    if start < items.len() {
        db.store_vector(table, &items[start..])?;
    }
    Ok(())
}
