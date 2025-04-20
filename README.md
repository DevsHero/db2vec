# db2vec: From Database Dumps to Vector Search at Speed

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Tired of waiting hours for Python scripts to embed large database exports? So was I. Processing millions of records demands performance that interpreted languages often struggle to deliver. That's why `db2vec` was born – a high‑performance Rust tool that parses your database dumps, generates vector embeddings via a local Ollama model, and loads them into your vector database of choice.

![db2vec CLI running](assets/db2vec_screenshot.png)

---

## Core Features

* 🚀 **Blazing Fast:** Built in Rust for maximum throughput on large datasets.  
* 📄 **Supported Dump Formats:**  
  - `.sql` (MySQL, PostgreSQL, MSSQL, SQLite, Oracle)  
    - **MSSQL:** Export via `sqlcmd` or `mssql-tools` into a plain `.sql` file:  
      ```bash
      sqlcmd -S server -U user -P pass -Q "SET NOCOUNT ON; SELECT * FROM dbo.TableName;" -o dump.sql
      ```  
    - *Oracle requires exporting via SQL Developer or similar into standard SQL.*  

  - `.surql` (SurrealDB)  
* 🧠 **Local Embeddings:** Uses Ollama (`EMBEDDING_MODEL`) to generate vectors.  
* 💾 **Vector DB Targets:** Inserts vectors + metadata into:  
  - Chroma  
  - Milvus  
  - Pinecone
  - Qdrant  
  - Redis Stack  
  - SurrealDB  
* ⚙️ **Pure Regex Parsing:** Fast, reliable record extraction (no AI).  
* 🔧 **Configurable:** CLI args + `.env` support (see Configuration).  
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
EMBEDDING_URL="http://localhost:11434/api/embeddings"
EMBEDDING_MODEL="nomic-embed-text-384-v2"
HOST="redis://127.0.0.1:6379"
DATABASE="default_database"
COLLECTION="my_collection"
DIMENSION=384
TENANT="default_tenant"
NAMESPACE="default_namespace"
METRIC="cosine"
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
     --collection mycol \
     --dimension 768 \
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
     --collection my-index \
     --namespace myns \
     --dimension 384 \
     --embedding-model nomic-embed-text-384-v2 \
     --metric cosine \
     -k <API_KEY> \
     --use-auth

   # SQLite → Redis
   ./target/release/db2vec \
     -f sqlite_dump.sql \
     -t redis \
     --host redis://127.0.0.1:6379
   ```

---

## Usage

```bash
# Cargo
cargo run -- [OPTIONS]

# Binary
./target/release/db2vec [OPTIONS]
```

**Options:**

* `-f, --data-file <FILE>`      Path to `.sql`/`.surql` dump [default: `./surreal.surql`]
* `-t, --db-export-type <TYPE>` `redis|chroma|milvus|qdrant|surreal|pinecone` [default: `redis`]
* `--host <HOST>`               Target DB URL/host [env: `HOST`]
* `--database <DATABASE>`       (Chroma/SurrealDB) [env: `DATABASE`]
* `--collection <COLLECTION>`   (Milvus, Qdrant, Chroma, Pinecone) [env: `COLLECTION`]
* `--dimension <DIMENSION>`     Vector dim (must match model) [env: `DIMENSION`]
* `--embedding-model <MODEL>`   Ollama model name [env: `EMBEDDING_MODEL`]
* `--metric <METRIC>`           (Pinecone) `cosine|euclidean|dotproduct` [env: `METRIC`]
* `--tenant <TENANT>`           (Chroma) [env: `TENANT`]
* `--namespace <NAMESPACE>`     (SurrealDB, Pinecone) [env: `NAMESPACE`]
* `--use-auth`                  Enable auth for DB
* `-u, --user <USER>`           Username (Milvus, SurrealDB)
* `-p, --pass <PASS>`           Password (Milvus, SurrealDB, Redis)
* `-k, --secret <SECRET>`       API key/token (Chroma, Qdrant, Pinecone)
* `--debug`                     Print parsed records
* `-h, --help`                  Show help
* `-V, --version`               Show version

---

## Docker Setup

Run supported vector DBs locally via Docker – see [DOCKER_SETUP.md](DOCKER_SETUP.md) for commands.

---

## How It Works

1. **Read & Detect:** Load dump (`.sql`/`.surql`), detect SQL dialect or SurrealDB.  
2. **Parse (Regex):** Extract records and types.  
3. **Embed:** Call Ollama with `EMBEDDING_MODEL` to get vectors.  
4. **Store:** Insert into your vector DB with metadata.

---

## Target Environment

Primarily developed and tested against Docker‑hosted or cloud vector databases via RESTful APIs. Ensure your target is reachable from where you run `db2vec`.

---

## Contributing

Issues, PRs, and feedback welcome!

---

## License

MIT – see [LICENSE](LICENSE).