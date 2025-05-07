use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the .sql/.surql database dump file to process
    #[arg(short = 'f', env = "DUMP_FILE", long, default_value = "./surreal.surql")]
    pub dump_file: String,

    /// Target vector database: redis|chroma|milvus|qdrant|surreal|pinecone
    #[arg(short = 't', env = "EXPORT_TYPE", long, default_value = "redis")]
    pub vector_export_type: String,

    /// Username for database authentication (Milvus, SurrealDB)
    #[arg(short = 'u', env = "USER", long, default_value = "root")]
    pub user: String,

    /// Password for database authentication (Milvus, SurrealDB, Redis)
    #[arg(short = 'p', env = "PASS", long, default_value = "")]
    pub pass: String,

    /// API key/token for database authentication (Chroma, Qdrant, Pinecone)
    #[arg(short = 'k', env = "SECRET", long, default_value = "")]
    pub secret: String,

    /// Enable authentication for the vector database
    #[arg(long, env = "AUTH", default_value = "false")]
    pub use_auth: bool,

    /// Print parsed JSON records before embedding (debug mode)
    #[arg(long, env = "DEBUG", default_value = "false")]
    pub debug: bool,

    /// Vector database URL/host endpoint (e.g. redis://127.0.0.1:6379)
    #[arg(long, env = "VECTOR_HOST", default_value = "redis://127.0.0.1:6379")]
    pub vector_host: String,

    /// Target database name (Chroma, Milvus, Qdrant, Surreal)
    #[arg(long, env = "DATABASE", default_value = "default_database")]
    pub database: String,

    /// Pinecone index name (only for -t pinecone)
    #[arg(long, env = "INDEXES", default_value = "default_indexes")]
    pub indexes: String,

    /// Pinecone cloud provider: aws|azure|gcp
    #[arg(long, env = "CLOUD", default_value = "aws")]
    pub cloud: String,

    /// Pinecone cloud region, e.g. us-east-1
    #[arg(long, env = "REGION", default_value = "us-east-1")]
    pub region: String,

    /// Tenant name for multi-tenant DBs (Chroma)
    #[arg(long, env = "TENANT", default_value = "default_tenant")]
    pub tenant: String,

    /// Namespace for databases that support it (SurrealDB, Pinecone)
    #[arg(long, env = "NAMESPACE", default_value = "default_namespace")]
    pub namespace: String,

    /// Vector dimension size (must match your embedding model)
    #[arg(long, env = "DIMENSION", default_value = "768")]
    pub dimension: usize,

    /// Distance metric: l2|ip|cosine|euclidean|dotproduct
    #[arg(long, env = "METRIC", default_value = "cosine")]
    pub metric: String,

    /// Max payload size (MB) per request
    #[arg(short = 'm', env = "PAYLOAD_SIZE_MB", long, default_value = "12")]
    pub max_payload_size_mb: usize,

    /// Batch size for DB inserts
    #[arg(short = 'c', env = "CHUNK_SIZE", long, default_value = "10")]
    pub chunk_size: usize,

    /// Which embedding provider to use: ollama, tei, or google
    #[arg(long, env = "EMBEDDING_PROVIDER", default_value = "ollama")]
    pub embedding_provider: String,

    /// API Key for Google Gemini (required if --embedding-provider=google)
    #[arg(long, env = "EMBEDDING_API_KEY")]
    pub embedding_api_key: Option<String>,

    /// Embedding model name/id (e.g. nomic-embed-text, text-embedding-004, nomic-embed-text-v2-moe)
    #[arg(long, env = "EMBEDDING_MODEL", default_value = "nomic-embed-text")]
    pub embedding_model: String,

    /// URL endpoint for Ollama or Google embeddings
    #[arg(long, env = "EMBEDDING_URL")]
    pub embedding_url: Option<String>,

    /// Parallel embedding requests
    #[arg(long, env = "EMBEDDING_MAX_CONCURRENCY", default_value = "4")]
    pub embedding_concurrency: usize,

    /// Number of texts per embedding batch
    #[arg(long, env = "EMBEDDING_BATCH_SIZE", default_value = "16")]
    pub embedding_batch_size: usize,

    /// Max tokens per embedding request (provider-specific)
    #[arg(long, env = "EMBEDDING_MAX_TOKENS", default_value = "8192")]
    pub embedding_max_tokens: usize,

    /// Timeout (seconds) for embedding calls
    #[arg(long, env = "OLLAMA_TIMEOUT", default_value = "60")]
    pub embedding_timeout: u64,

    /// Task type for Google Gemini (default: SEMANTIC_SIMILARITY)
    #[arg(long, env = "EMBEDDING_TASK_TYPE", default_value = "SEMANTIC_SIMILARITY")]
    pub embedding_task_type: String,

    /// CPU threads for parallel tasks (0 = auto detect)
    #[arg(long, env = "NUM_THREADS", default_value = "0")]
    pub num_threads: usize,

    /// Group Redis records by table name if true (else use FT.CREATE/SEARCH)
    #[arg(long, env = "GROUP_REDIS", default_value = "false")]
    pub group_redis: bool,

    /// Path to TEI binary (tei-metal or tei-onnx).  
    /// If you omit this, the embedded TEI will be extracted & launched.
    #[arg(long, env = "TEI_BINARY_PATH", default_value = "tei/tei-metal")]
    pub tei_binary_path: String,

    /// Port for the managed TEI server (only used if starting TEI locally)
    #[arg(long, env = "TEI_LOCAL_PORT", default_value_t = 8080)]
    pub tei_local_port: u16,

    /// Apply exclusion rules from config/exclude.json to remove sensitive fields
    #[arg(long, env = "USE_EXCLUDE", default_value = "false")]
    pub use_exclude: bool,
}
