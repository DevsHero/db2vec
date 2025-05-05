# db2vec Command-Line Options

Below is the full list of CLI flags, their environment-variable equivalents, defaults, and descriptions.  
(Note: `--tei-local-port` has been removed; only `--tei-binary-path` remains.)

| Flag / Env Var                                      | Default                  | Description                                                                                   |
|-----------------------------------------------------|--------------------------|-----------------------------------------------------------------------------------------------|
| -f, --data-file <FILE> <br> DUMP_FILE               | `./surreal.surql`        | Path to the `.sql` / `.surql` database-dump file.                                             |
| -t, --vector-export-type <EXPORT_TYPE> <br> EXPORT_TYPE               | `redis`                  | Target vector database: `redis` \| `chroma` \| `milvus` \| `qdrant` \| `surreal` \| `pinecone`.|
| -u, --user <USER> <br> USER                         | `root`                   | Username for DB authentication (Milvus, SurrealDB).                                           |
| -p, --pass <PASS> <br> PASS                         | `""`                     | Password for DB authentication (Milvus, SurrealDB, Redis).                                    |
| -k, --secret <SECRET> <br> SECRET                   | `""`                     | API key / token for DB auth (Chroma, Qdrant, Pinecone).                                       |
| --use-auth <BOOL> <br> AUTH                         | `false`                  | Enable authentication for the vector database.                                                |
| --debug <BOOL> <br> DEBUG                           | `false`                  | Print parsed JSON records before embedding.                                                   |
| --vector-host <HOST> <br> VECTOR_HOST               | `redis://127.0.0.1:6379` | Vector-database URL or host endpoint.                                                         |
| --database <DB> <br> DATABASE                       | `default_database`       | Target database/collection name (Chroma, Milvus, Qdrant, Surreal).                           |
| --indexes <NAME> <br> INDEXES                       | `default_indexes`        | Pinecone index name (only for `-t pinecone`).                                                 |
| --cloud <CLOUD> <br> CLOUD                          | `aws`                    | Pinecone cloud provider: `aws` \| `azure` \| `gcp`.                                           |
| --region <REGION> <br> REGION                       | `us-east-1`              | Pinecone cloud region (e.g. `us-east-1`).                                                     |
| --tenant <TENANT> <br> TENANT                       | `default_tenant`         | Tenant name for multi-tenant DBs (Chroma).                                                    |
| --namespace <NAMESPACE> <br> NAMESPACE              | `default_namespace`      | Namespace for SurrealDB or Pinecone.                                                          |
| --dimension <N> <br> DIMENSION                      | `768`                    | Vector dimension size (must match your embedding model).                                      |
| --metric <METRIC> <br> METRIC                       | `cosine`                 | Distance metric: `l2` \| `ip` \| `cosine` \| `euclidean` \| `dotproduct`.                    |
| -m, --max-payload-size-mb <MB> <br> PAYLOAD_SIZE_MB | `12`                     | Max payload size **MB** per request (DB batch upload).                                        |
| -c, --chunk-size <N> <br> CHUNK_SIZE                | `10`                     | Number of records per batch insert.                                                           |
| --embedding-provider <PROVIDER> <br> EMBEDDING_PROVIDER | `ollama`               | Embedding provider: `ollama` (fast CPU/GPU) \| `tei` (CPU-only TEI v1.7.0) \| `google` (cloud).|
| --embedding-api-key <KEY> <br> EMBEDDING_API_KEY    | _none_                   | API Key for Google Gemini (required if provider=`google`).                                     |
| --embedding-model <MODEL> <br> EMBEDDING_MODEL      | `nomic-embed-text`       | Model name/ID for your provider (e.g. `nomic-embed-text`, `text-embedding-004`, `...-moe`).   |
| --embedding-url <URL> <br> EMBEDDING_URL            | _none_                   | URL endpoint for Ollama or Google embeddings (e.g. `http://localhost:11434`).                |
| --embedding-max-concurrency <N> <br> EMBEDDING_MAX_CONCURRENCY | `4`             | Parallel embedding requests.                                                                  |
| --embedding-batch-size <N> <br> EMBEDDING_BATCH_SIZE | `16`                     | Number of texts per embedding batch.                                                          |
| --embedding-max-tokens <N> <br> EMBEDDING_MAX_TOKENS | `8000`                   | Max tokens per embedding request (provider-specific).                                         |
| --embedding-timeout <SEC> <br> OLLAMA_TIMEOUT       | `60`                     | Timeout (seconds) for embedding calls.                                                        |
| --embedding-task-EXPORT_TYPE <EXPORT_TYPE> <br> EMBEDDING_TASK_EXPORT_TYPE | `SEMANTIC_SIMILARITY` | Optional task EXPORT_TYPE for Google Gemini API.                                                     |
| --num-threads <N> <br> NUM_THREADS                  | `0`                      | CPU threads for parallel tasks (0 = auto-detect).                                             |
| --group-redis <BOOL> <br> GROUP_REDIS               | `false`                  | Group Redis records by table name (vs individual FT.CREATE/SEARCH).                           |
| --tei-binary-path <PATH> <br> TEI_BINARY_PATH       | `tei/tei-metal`          | Path to TEI binary (`tei-metal` or `tei-onnx`). If omitted, the embedded TEI is auto-extracted.| 


This document now reflects the removal of `--tei-local-port` and clearly lists the remaining CLI options, including how to invoke and configure the TEI binary.This document now reflects the removal of `--tei-local-port` and clearly lists the remaining CLI options, including how to invoke and configure the TEI binary.