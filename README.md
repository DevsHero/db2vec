# db2vec: From Database Dumps to Vector Search at Speed

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Tired of waiting hours for Python scripts to embed large database exports? So was I. Processing millions of records demands performance that interpreted languages often struggle to deliver. That's why `db2vec` was born – a high‑performance Rust tool that parses your database dumps, generates vector embeddings via a local Ollama model (in parallel!), and loads them into your vector database of choice.

![db2vec CLI running](assets/db2vec_screenshot.png)

---

## Core Features

* 🚀 **Blazing Fast:** Built in Rust for maximum throughput on large datasets.
* 🔄 **Parallel Processing:** Adjustable concurrency and batch‑size for embedding generation (`--num‑threads`, `--embedding‑concurrency`, `--embedding‑batch‑size`).
* 📦 **Batch Inserts:** Configurable batch size (`-b, --batch-size-mb`) for efficient bulk loading into the target vector database.
* 🔧 **Highly Configurable:** Fine-tune performance and behavior with extensive CLI arguments for embedding, database connections, batching, and more.
* 📄 **Supported Dump Formats:**
  - `.sql` (MySQL, PostgreSQL, MSSQL, SQLite, Oracle)
    - **MSSQL:**
      ```bash
      sqlcmd -S server -U user -P pass -Q "SET NOCOUNT ON; SELECT * FROM dbo.TableName;" -o dump.sql
      ```
    - *Oracle requires exporting via SQL Developer or similar into standard SQL.*
  - `.surql` (SurrealDB)
* 🧠 **Local Embeddings:** Uses Ollama (`--embedding-model`) to generate vectors.
* 💾 **Vector DB Targets:** Inserts vectors + metadata into:
  - Chroma
  - Milvus
  - Pinecone
  - Qdrant
  - Redis Stack
  - SurrealDB
* ⚙️ **Pure Regex Parsing:** Fast, reliable record extraction (no AI).
* 🔒 **Authentication:** Supports user/password, API key, tenants/namespaces per DB.
* 🐞 **Debug Mode:** `--debug` prints parsed JSON records before embedding.

---

## Requirements

* **Rust:** Latest stable (Edition 2021+).  
* **Ollama:** Running locally with your model(s):  
  ```bash
  ollama pull nomic-embed-text        # 768‑dim
  ollama pull nomic-embed-text-384-v2 # 384‑dim
  ```  
* **Target DB:** One of Chroma, Milvus, Pinecone, Qdrant, Redis Stack, SurrealDB (Docker recommended).  
* **(Optional) `.env`:** For embedding URL/model and other defaults.

---

## Configuration

Use CLI flags or `.env` (CLI always wins).  

```env
# File and database type options
FILE_PATH="./surreal.surql"
TYPE="redis"

# Authentication
USER="root"
PASS=""
SECRET=""
AUTH="false"

# Connection details
HOST="redis://127.0.0.1:6379"
DATABASE="default_database"
TENANT="default_tenant"
NAMESPACE="default_namespace"

# Vector settings
DIMENSION=768
METRIC="cosine"

# Batch processing options
PAYLOAD_SIZE_MB=12
CHUNK_SIZE=10

# Embedding configuration
EMBEDDING_URL="http://localhost:11434"
EMBEDDING_MODEL="nomic-embed-text"
EMBEDDING_MAX_CONCURRENCY=4
EMBEDDING_BATCH_SIZE=16
EMBEDDING_MAX_TOKENS=8000
OLLAMA_TIMEOUT=60

# Performance settings
NUM_THREADS=0

# Special behaviors
DEBUG="false"
GROUP_REDIS="false"
```

---

## Quick Start

1. **Clone & build**  
   ```bash
   git clone https://github.com/DevsHero/db2vec.git
   cd db2vec
   cargo build --release
   ```  
2. **Prepare your dump**  
   - MySQL/Postgres/Oracle: export `.sql`  
   - MSSQL: `sqlcmd … > mssql_dump.sql`  
   - SQLite: `sqlite3 mydb.db .dump > sqlite_dump.sql`  
   - SurrealDB: `.surql` file  
3. **Run**  
   ```bash
   # MySQL → Milvus
   ./target/release/db2vec \
     -f mysql_sample.sql \
     -t milvus \
     --host http://127.0.0.1:19530 \
     --database mydb \
     --dimension 768 \              # Must match nomic-embed-text's output dimension
     --embedding-model nomic-embed-text \
     -u root \
     -p secret \
     --use-auth \
     --debug

   # MSSQL → Pinecone
   ./target/release/db2vec \
     -f mssql_dump.sql \
     -t pinecone \
     --host <INDEX_HOST> \
     --namespace myns \
     --dimension 384 \              # Must match nomic-embed-text-384-v2's output dimension
     --embedding-model nomic-embed-text-384-v2 \
     --metric cosine \
     -k <API_KEY> \
     --use-auth

   # SQLite → Redis
   ./target/release/db2vec \
     -f sqlite_dump.sql \
     -t redis \
     --host redis://127.0.0.1:6379 \
     --group-redis  # Group records by table name
   ```

---

## Usage

```bash
# Cargo
cargo run -- [OPTIONS]

# Binary
./target/release/db2vec [OPTIONS]

# Log (info)
RUST_LOG=info 

# Debug  
RUST_LOG=debug  --debug
```

**Options:**

* `-f, --data-file <FILE>`               Path to `.sql`/`.surql` dump [default: `./surreal.surql`]
* `-t, --db-export-type <TYPE>`          `redis|chroma|milvus|qdrant|surreal|pinecone` [default: `redis`]
* `-u, --user <USER>`                    Username for DB auth [default: `root`]
* `-p, --pass <PASS>`                    Password for DB auth [default: `""`]
* `-k, --secret <SECRET>`                API key/token for DB auth [default: `""`]
* `--use-auth`                           Enable authentication [default: `false`]
* `--debug`                              Enable debug mode [default: `false`]
* `--host <HOST>`                        Target DB URL/host [env: `HOST`, default: `redis://127.0.0.1:6379`]
* `--database <DATABASE>`                Target DB name [env: `DATABASE`, default: `default_database`]
* `--tenant <TENANT>`                    (Chroma) Tenant name [env: `TENANT`, default: `default_tenant`]
* `--namespace <NAMESPACE>`              (SurrealDB/Pinecone) Namespace [env: `NAMESPACE`, default: `default_namespace`]
* `--dimension <DIMENSION>`              Vector dimension [env: `DIMENSION`, default: `768`]
* `--metric <METRIC>`                    Distance metric  example `ip|cosine|euclidean|dotproduct` [env: `METRIC`, default: `cosine`]
* `-m, --max-payload-size-mb <MB>`       Maximum payload size in MB for database requests [default: `12`]
* `-c, --chunk-size <N>`                 Number of items to process in each database batch [default: `10`]
* `--embedding-model <MODEL>`            Ollama model name [env: `EMBEDDING_MODEL`, default: `nomic-embed-text`]
* `--embedding-url <URL>`                Embedding API endpoint [env: `EMBEDDING_URL`, default: `http://localhost:11434`]
* `--embedding-max-concurrency <N>`      Max parallel embedding requests [env: `EMBEDDING_MAX_CONCURRENCY`, default: `4`]
* `--embedding-batch-size <N>`           Prompts per batch API call [env: `EMBEDDING_BATCH_SIZE`, default: `16`]
* `--embedding-max-tokens <N>`           Text truncation limit [env: `EMBEDDING_MAX_TOKENS`, default: `8000`]
* `--embedding-timeout <SEC>`            Ollama request timeout [env: `OLLAMA_TIMEOUT`, default: `60`]
* `--num-threads <N>`                    CPU threads for parallel processing (0=auto-detect) [env: `NUM_THREADS`, default: `0`]
* `--group-redis`                        Group Redis records by table name (vs storing as individual entries) [default: `false`]

---

## Docker Setup

Run supported vector DBs locally via Docker – see [DOCKER_SETUP.md](DOCKER_SETUP.md) for commands.

---

## How It Works

1. **Read & Detect:** Load dump (`.sql`/`.surql`), detect SQL dialect or SurrealDB.  
2. **Parse (Regex):** Extract records and types.  
3. **Embed:** Call Ollama with `EMBEDDING_MODEL` to get vectors.  
4. **Auto-Schema:** Automatically create:
   - Target database if it doesn't exist 
   - Collections/indices from table names in the dump
   - Proper dimension settings based on your `--dimension` parameter
   - Distance metrics using your specified `--metric` value
5. **Store:** Insert into your vector DB with metadata.

---

## Automatic Collection Creation

For each table in your source data dump, `db2vec` automatically:

* Creates a corresponding collection/index in the target vector database
* Names the collection after the source table name
* Configures proper dimensions and metric type based on your CLI arguments
* Creates the database first if it doesn't exist

This zero-config schema creation means you don't need to manually set up your vector database structure before import.

> **Note:** When using Redis with `--group-redis`, collections aren't created in the traditional sense. Instead, records are grouped by table name into Redis data structures (e.g., `table:profile` → [records]). Without this flag, Redis stores each record as an individual entry with a table label in the metadata.
> 
> **Warning:** If collections already exist, their dimension must match the `--dimension` parameter you provide. Some databases like Pinecone will reject vectors with mismatched dimensions, causing the import to fail.

---

## Target Environment

Primarily developed and tested against Docker‑hosted or cloud vector databases via RESTful APIs. Ensure your target is reachable from where you run `db2vec`.

---

## Contributing

Issues, PRs, and feedback welcome!

---

## License

MIT – see [LICENSE](LICENSE).