use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the .sql/.surql database dump file to process.
    #[arg(short = 'f', long, default_value = "./surreal.surql")]
    pub data_file: String,

    /// Type of vector database to connect to (e.g., redis, chroma, milvus, qdrant, surreal, pinecone).
    #[arg(short = 't', long, default_value = "redis")]
    pub db_export_type: String,

    /// Username for database authentication. (Milvus, SurrealDB)
    #[arg(short = 'u', long, default_value = "root")]
    pub user: String,

    /// Password for database authentication. (Milvus, SurrealDB, Redis)
    #[arg(short = 'p', long, default_value = "")]
    pub pass: String,

    /// Secret key or API token for database authentication (Chroma, Qdrant, Pinecone).
    #[arg(short = 'k', long, default_value = "")]
    pub secret: String,

    /// Flag to enable authentication (specific usage depends on the database type).
    #[arg(long, default_value = "false")]
    pub use_auth: bool,

    /// Enable debug mode to print parsed records after regex processing.
    #[arg(long, default_value = "false")]
    pub debug: bool,

    /// Host URL for the target vector database (e.g., redis://..., http://...).
    #[arg(long, env = "HOST", default_value = "redis://127.0.0.1:6379")]
    pub host: String,

    /// Target database name (Chroma, SurrealDB).
    #[arg(long, env = "DATABASE", default_value = "default_database")]
    pub database: String,

    /// Target collection/index name within the vector database. (Milvus, Qdrant, Chroma, Pinecone)
    #[arg(long, env = "COLLECTION", default_value = "my_collection")]
    pub collection: String,

    /// Tenant name (used by some databases like Chroma).
    #[arg(long, env = "TENANT", default_value = "default_tenant")]
    pub tenant: String,

    /// Namespace (used by some databases like SurrealDB, Pinecone).
    #[arg(long, env = "NAMESPACE", default_value = "default_namespace")]
    pub namespace: String,

    /// Dimension size of the vectors being stored. (Milvus, Qdrant, Chroma, Pinecone)
    #[arg(long, env = "DIMENSION", default_value = "768")]
    pub dimension: usize,

    /// Distance metric for vector similarity (Pinecone: cosine, euclidean, dotproduct).
    #[arg(long, env = "METRIC", default_value = "cosine")]
    pub metric: String,
}
