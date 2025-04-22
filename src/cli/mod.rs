use clap::Parser;

#[derive(Parser, Debug, Clone)] // Add Clone here
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the .sql/.surql database dump file to process
    #[arg(short = 'f', long, default_value = "./surreal.surql")]
    pub data_file: String,

    /// Vector database type (Redis, Chroma, Milvus, Qdrant, Surreal, Pinecone)
    #[arg(short = 't', long, default_value = "redis")]
    pub db_export_type: String,

    /// Username for database authentication (Milvus, SurrealDB)
    #[arg(short = 'u', long, default_value = "root")]
    pub user: String,

    /// Password for database authentication (Milvus, SurrealDB, Redis)
    #[arg(short = 'p', long, default_value = "")]
    pub pass: String,

    /// API key/token for database authentication (Chroma, Qdrant, Pinecone)
    #[arg(short = 'k', long, default_value = "")]
    pub secret: String,

    /// Enable authentication for the vector database
    #[arg(long, default_value = "false")]
    pub use_auth: bool,

    /// Enable debug mode to print parsed records
    #[arg(long, default_value = "false")]
    pub debug: bool,

    /// Vector database URL/host endpoint
    #[arg(long, env = "HOST", default_value = "redis://127.0.0.1:6379")]
    pub host: String,

    /// Target database name
    #[arg(long, env = "DATABASE", default_value = "default_database")]
    pub database: String,

    /// Collection/index name within database
    #[arg(long, env = "COLLECTION", default_value = "my_collection")]
    pub collection: String,

    /// Tenant name for multi-tenant databases (Chroma)
    #[arg(long, env = "TENANT", default_value = "default_tenant")]
    pub tenant: String,

    /// Namespace for databases that support it (SurrealDB, Pinecone)
    #[arg(long, env = "NAMESPACE", default_value = "default_namespace")]
    pub namespace: String,

    /// Vector dimension size
    #[arg(long, env = "DIMENSION", default_value = "768")]
    pub dimension: usize,

    /// Distance metric for vector similarity (cosine, euclidean, dotproduct)
    #[arg(long, env = "METRIC", default_value = "cosine")]
    pub metric: String,

    /// Maximum payload size in MB for vector database requests
    #[arg(
        short = 'm',
        long,
        default_value = "12",
        help = "Maximum payload size in MB for database requests"
    )]
    pub max_payload_size_mb: usize,

    /// Number of chunks to process in parallel for storage
    #[arg(short = 'c', long, default_value = "10")]
    pub chunk_size: usize, // Fixed typo: chuck_size -> chunk_size

    /// Embedding model to use with Ollama
    #[arg(long, env = "EMBEDDING_MODEL", default_value = "nomic-embed-text")]
    pub embedding_model: String,

    /// Embedding API endpoint URL
    #[arg(long, env = "EMBEDDING_URL", default_value = "http://localhost:11434")]
    pub embedding_url: String,

    /// Maximum parallel embedding requests
    #[arg(long, env = "EMBEDDING_MAX_CONCURRENCY", default_value = "4")]
    pub embedding_concurrency: usize,

    /// Number of texts per embedding batch request
    #[arg(long, env = "EMBEDDING_BATCH_SIZE", default_value = "16")]
    pub embedding_batch_size: usize,

    /// Maximum tokens for text truncation
    #[arg(long, env = "EMBEDDING_MAX_TOKENS", default_value = "8000")]
    pub embedding_max_tokens: usize,

    /// Timeout in seconds for embedding requests
    #[arg(long, env = "OLLAMA_TIMEOUT", default_value = "60")]
    pub embedding_timeout: u64,

    /// CPU threads for parallel processing (0=auto-detect)
    #[arg(long, env = "NUM_THREADS", default_value = "0")]
    pub num_threads: usize,

    /// Enable Redis grouping of records by table name.
    /// If true, records will be grouped by table name ("table:profile" -> [records]).
    /// If false, records will be stored as unique entries with a table label inserted
    /// into the JSON (46ef6eb2-a222-486f-a869-6c220a898758 -> {label: "table:profile"}).
    #[arg(long, default_value = "false")]
    pub group_redis: bool,
}
