# db2vec: From Database Dumps to Vector Search at Speed (CPU Focused)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Tired of waiting hours for Python scripts to embed large database exports, especially on machines without powerful GPUs? So was I. Processing millions of records demands performance, even on standard hardware. `db2vec` is a high‑performance Rust tool designed for efficient **CPU-based embedding generation**. It parses your database dumps, generates vector embeddings using local models (Ollama, Rust-Bert) or cloud APIs (Google Gemini), and loads them into your vector database of choice – all optimized for speed without requiring a dedicated GPU.

![db2vec CLI running](assets/db2vec_screenshot.png)

---

## Core Features

*   🚀 **Blazing Fast:** Built in Rust for maximum throughput on large datasets, optimized for CPU.
*   🔄 **Parallel Processing:** Adjustable concurrency and batch‑size for embedding generation (`--num‑threads`, `--embedding‑concurrency`, `--embedding‑batch-size`).
*   📦 **Batch Inserts:** Configurable batch size (`-c, --chunk-size`) and payload limits (`-m, --max-payload-size-mb`) for efficient bulk loading into the target vector database.
*   🔧 **Highly Configurable:** Fine-tune performance and behavior with extensive CLI arguments for embedding, database connections, batching, and more.
*   📄 **Supported Dump Formats:**
    *   `.sql` (MySQL, PostgreSQL, MSSQL, SQLite, Oracle)
        *   **MSSQL:**
            ```bash
            sqlcmd -S server -U user -P pass -Q "SET NOCOUNT ON; SELECT * FROM dbo.TableName;" -o dump.sql
            ```
        *   *Oracle requires exporting via SQL Developer or similar into standard SQL.*
    *   `.surql` (SurrealDB)
*   🧠 **Flexible Embeddings:** Supports multiple providers:
    *   **Ollama:** Use any compatible model running locally.
    *   **Rust-Bert:** Built-in support for `all-MiniLM-L6-v2` (384-dim), running efficiently on CPU.
    *   **Google Gemini:** Use models like `text-embedding-004` via API key.
*   💾 **Vector DB Targets:** Inserts vectors + metadata into:
    *   Chroma
    *   Milvus
    *   Pinecone (Cloud & Local Dev Image)
    *   Qdrant
    *   Redis Stack
    *   SurrealDB
*   ⚙️ **Pure Regex Parsing:** Fast, reliable record extraction (no AI).
*   🔒 **Authentication:** Supports user/password, API key, tenants/namespaces per DB.
*   ☁️ **Pinecone Cloud Support:** Automatically creates/describes indexes, uses namespaces.
*   🐞 **Debug Mode:** `--debug` prints parsed JSON records before embedding.

---

## Requirements

*   **Rust:** Latest stable (Edition 2021+).
*   **Embedding Provider:** One of the following configured:
    *   **Ollama:** Running locally with your desired model(s) pulled (e.g., `ollama pull nomic-embed-text`).
    *   **Rust-Bert:** No extra setup needed for the default `all-MiniLM-L6-v2` model; it runs directly on the CPU using bundled libraries.
    *   **Google Gemini:** A valid Google Cloud API key (`--secret` or `EMBEDDING_API_KEY`) with the Generative Language API enabled for your project.
*   **Target DB:** One of Chroma, Milvus, Pinecone, Qdrant, Redis Stack, SurrealDB (Docker recommended for local).
*   **(Optional) `.env`:** For setting default configuration values.

---

## Configuration

Configuration can be set using CLI flags or by creating a `.env` file in the project root. CLI flags always override values set in the `.env` file.

Refer to the `.env-example` file for a comprehensive list of available environment variables, their descriptions, and default values.

---

## Quick Start

1.  **Clone & build**
    ```bash
    git clone https://github.com/DevsHero/db2vec.git
    cd db2vec
    cargo build --release
    ```
2.  **Prepare your dump**
    *   MySQL/Postgres/Oracle: export `.sql`
    *   MSSQL: `sqlcmd … > mssql_dump.sql`
    *   SQLite: `sqlite3 mydb.db .dump > sqlite_dump.sql`
    *   SurrealDB: `.surql` file
3.  **(Optional) Create `.env`:** Copy `.env-example` to `.env` and customize defaults.
4.  **Run**
    ```bash
    # MySQL → Milvus (using Ollama)
    ./target/release/db2vec \
      -f mysql_sample.sql \
      -t milvus \
      --host http://127.0.0.1:19530 \
      --database mydb \
      --embedding-provider ollama \
      --embedding-model nomic-embed-text \
      --dimension 768 \
      -u root -p secret --use-auth \
      --debug

    # MSSQL → Pinecone (using Google Gemini)
    ./target/release/db2vec \
      -f mssql_dump.sql \
      -t pinecone \
      --host <INDEX_HOST> \
      --namespace myns \
      --embedding-provider google \
      --embedding-model text-embedding-004 \
      --dimension 768 \
      --metric cosine \
      --embedding-api-key <GOOGLE_API_KEY> \ # Use specific key arg
      --use-auth

    # SQLite → Qdrant (using built-in Rust-Bert on CPU)
    ./target/release/db2vec \
      -f sqlite_dump.sql \
      -t qdrant \
      --host http://localhost:6333 \
      --embedding-provider rustbert \
      --dimension 384 \ # Must be 384 for the default all-MiniLM-L6-v2
      --metric cosine
    ```

---

## Usage

```bash
# Cargo
cargo run -- [OPTIONS]

# Binary
./target/release/db2vec [OPTIONS]

# Logging
RUST_LOG=info ./target/release/db2vec [OPTIONS]
RUST_LOG=debug ./target/release/db2vec --debug [OPTIONS]
```

**Options:**

*   `-f, --data-file <FILE>` Path to the `.sql`/`.surql` dump [env: FILE_PATH, default: `./surreal.surql`]
*   `-t, --db-export-type <TYPE>` Target DB type: `redis|chroma|milvus|qdrant|surreal|pinecone` [env: TYPE, default: `redis`]
*   `-u, --user <USER>` Username for DB auth (Milvus, SurrealDB) [env: USER, default: `root`]
*   `-p, --pass <PASS>` Password for DB auth (Milvus, SurrealDB, Redis) [env: PASS, default: `""`]
*   `-k, --secret <SECRET>` API key / token (Chroma, Qdrant, Pinecone) [env: SECRET, default: `""`]
*   `--use-auth` Enable authentication for the vector database [env: AUTH, default: `false`]
*   `--debug` Enable debug mode (prints parsed JSON) [env: DEBUG, default: `false`]
*   `--host <HOST>` DB URL / host endpoint.
    – Redis: `redis://127.0.0.1:6379`
    – Pinecone Cloud: full data‑plane URL (e.g. `https://index‑123.svc.us‑east‑1.pinecone.io`)
    [env: HOST, default: `redis://127.0.0.1:6379`]
*   `--database <DATABASE>` Target database name (Chroma, Milvus, Qdrant, Surreal) [env: DATABASE, default: `default_database`]
*   `--indexes <INDEXES>` Pinecone index name (will be created/described on Cloud) [env: INDEXES, default: `default_indexes`]
*   `--cloud <CLOUD>` Pinecone cloud provider: `aws|azure|gcp` [env: CLOUD, default: `aws`]
*   `--region <REGION>` Pinecone cloud region (e.g. `us-east-1`) [env: REGION, default: `us-east-1`]
*   `--tenant <TENANT>` Chroma multi-tenant name [env: TENANT, default: `default_tenant`]
*   `--namespace <NAMESPACE>` Namespace for databases that support it (SurrealDB, Pinecone) [env: NAMESPACE, default: `default_namespace`]
*   `--dimension <DIMENSION>` Vector dimension size (must match embedding model output!) [env: DIMENSION, default: `768`]
*   `--metric <METRIC>` Distance metric: `l2|ip|cosine|euclidean|dotproduct` [env: METRIC, default: `cosine`]
*   `-m, --max-payload-size-mb <MB>` Max payload size in MB [env: PAYLOAD_SIZE_MB, default: `12`]
*   `-c, --chunk-size <N>` Number of records per batch insert [env: CHUNK_SIZE, default: `10`]
*   `--embedding-provider <PROVIDER>` Embedding provider: `ollama|rustbert|google` [env: EMBEDDING_PROVIDER, default: `ollama`]
*   `--embedding-api-key <KEY>` API Key for Google Gemini (required if provider is 'google') [env: EMBEDDING_API_KEY]
*   `--embedding-model <MODEL>` Model name (Ollama, Google) or path (local Rust-Bert override) [env: EMBEDDING_MODEL, default: `nomic-embed-text`]
*   `--embedding-url <URL>` API endpoint for Ollama or Google [env: EMBEDDING_URL, default: `http://localhost:11434`]
*   `--embedding-task-type <TYPE>` Optional task type for Google Gemini API [env: EMBEDDING_TASK_TYPE, default: `SEMANTIC_SIMILARITY`]
*   `--embedding-max-concurrency <N>` Parallel embedding requests [env: EMBEDDING_MAX_CONCURRENCY, default: `4`]
*   `--embedding-batch-size <N>` Texts per embedding batch [env: EMBEDDING_BATCH_SIZE, default: `16`]
*   `--embedding-max-tokens <N>` Max tokens per embedding request (provider specific) [env: EMBEDDING_MAX_TOKENS, default: `8000`]
*   `--embedding-timeout <SEC>` Embedding timeout in seconds [env: OLLAMA_TIMEOUT, default: `60`]
*   `--num-threads <N>` CPU threads for parallel tasks (0=auto‑detect) [env: NUM_THREADS, default: `0`]
*   `--group-redis` Group Redis records by table name (vs individual keys) [env: GROUP_REDIS, default: `false`]

---

## Pinecone Cloud Support

When `-t pinecone` is selected and `--host` is not a local URL:

1.  **Create / Describe Index**
    *   Uses the control plane `https://api.pinecone.io/indexes`
    *   Requires `--indexes`, `--secret` (API key), `--cloud`, and `--region`
    *   If the index does not exist, it is created with your `--dimension` and `--metric`
    *   On `409 Conflict`, the existing index is described to retrieve its data‑plane host

2.  **Data‑Plane Upserts**
    *   Vectors are upserted to `https://<your-index-host>`
    *   Namespace = source table name (each table is a separate namespace)
    *   Metadata includes a `"table": "<table_name>"` field

> **Note:** For local Pinecone dev images, index creation via API may not be supported.
> Ensure your index exists or provide the full data‑plane URL with `--host`.

---

## Docker Setup

Run supported vector DBs locally via Docker – see [DOCKER_SETUP.md](DOCKER_SETUP.md) for commands.

---

## How It Works

1.  **Read & Detect:** Load dump (`.sql`/`.surql`), detect SQL dialect or SurrealDB.
2.  **Parse (Regex):** Extract records and types.
3.  **Embed:** Call the selected embedding provider (`ollama`, `rustbert` on CPU, `google`) to get vectors.
4.  **Auto-Schema:** Automatically create:
    *   Target database if it doesn't exist
    *   Collections/indices from table names in the dump
    *   Proper dimension settings based on your `--dimension` parameter
    *   Distance metrics using your specified `--metric` value
5.  **Store:** Insert into your vector DB with metadata.

---

## Automatic Collection Creation

For each table in your source data dump, `db2vec` automatically:

*   Creates a corresponding collection/index in the target vector database
*   Names the collection after the source table name
*   Configures proper dimensions and metric type based on your CLI arguments
*   Creates the database first if it doesn't exist

This zero-config schema creation means you don't need to manually set up your vector database structure before import.

> **Note:** When using Redis with `--group-redis`, collections aren't created in the traditional sense. Instead, records are grouped by table name into Redis data structures (e.g., `table:profile` → [records]). Without this flag, Redis stores each record as an individual entry with a table label in the metadata.
>
> **Warning:** If collections already exist, their dimension must match the `--dimension` parameter you provide. Some databases like Pinecone will reject vectors with mismatched dimensions, causing the import to fail.

---

## Target Environment

Primarily developed and tested against Docker‑hosted or cloud vector databases via RESTful APIs. Ensure your target is reachable from where you run `db2vec`. **Designed to run efficiently even on standard CPU hardware.**

---

## Contributing

Issues, PRs, and feedback welcome!

---

## License

MIT – see [LICENSE](LICENSE).