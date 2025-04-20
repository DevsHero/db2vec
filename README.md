# db2vec: From Database Dumps to Vector Search at Speed

 
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Tired of waiting hours for Python scripts to embed large database exports? So was I. Processing millions of records demands performance that interpreted languages often struggle to deliver. That's why `db2vec` was born – a high-performance Rust tool designed to bridge the gap between your traditional database dumps and the world of vector search, quickly and efficiently.

`db2vec` parses common database export formats, generates vector embeddings using local Ollama models, and loads the data into your favorite vector database, all optimized for speed.

![db2vec CLI running](assets/db2vec_screenshot)

---

## Core Features

*   🚀 **Blazing Fast:** Built in Rust for maximum performance on large datasets.
*   📄 **Supported Dump Formats:** Natively parses:
    *   `.sql` (MySQL, PostgreSQL, Oracle* )
    *   `.surql` (SurrealDB)
    *   *Oracle requires export via SQL Developer or similar tool into standard SQL format.*
    *   *JSON detection as a fallback.*
*   **Rich Type Handling:** Parses and preserves various data types from dumps, including strings, numbers, booleans, NULLs, arrays, and nested JSON objects where supported by the format.
*   🧠 **Local Embeddings:** Integrates seamlessly with [Ollama](https://ollama.com/) to generate embeddings using your chosen models (e.g., `nomic-embed-text`).
*   💾 **Vector DB Targets:** Stores data and vectors in popular vector databases:
    *   Chroma
    *   Milvus
    *   Redis Stack
    *   SurrealDB
    *   Qdrant
*   ⚙️ **Pure Regex Power:** Utilizes optimized regular expressions for reliable and fast parsing across supported formats. **No AI parsing involved** for maximum speed.
*   🔧 **Configurable:** Flexible configuration via CLI arguments and a simple `.env` file.
*   🔒 **Authentication:** Supports various authentication methods for target databases.
*   🐞 **Debug Mode:** Optional verbose output for inspecting parsed records.

---

## Requirements

*   **Rust:** Latest stable version (Edition 2021+).
*   **Ollama:** Running locally. [Install Ollama](https://ollama.com/)
    *   Pull your desired embedding model (e.g., `nomic-embed-text`):
        ```bash
        ollama pull nomic-embed-text
        ```
*   **Target Vector Database:** An instance of Chroma, Milvus, Redis Stack, SurrealDB, or Qdrant running (Docker recommended, tested via REST APIs).
*   **(Optional) `.env` File:** For setting Ollama connection details.

---

## Configuration

Configure `db2vec` using command-line arguments (see Usage) or by creating a `.env` file in the project root. CLI arguments always override `.env` variables.

**`.env` Example:**

```env
# --- Ollama Embedding Configuration ---
# URL of your Ollama API endpoint
EMBEDDING_URL="http://localhost:11434/api/embeddings"
# Name of the embedding model pulled in Ollama
EMBEDDING_MODEL="nomic-embed-text"

# --- Other Optional Defaults (Can be set via CLI) ---
# HOST="http://localhost:8000"
# DATABASE="default_database"
# COLLECTION="my_collection"
# DIMENSION=768
# TENANT="default_tenant"
# NAMESPACE="default_namespace"
```

---

## Quick Start

1.  **Clone the Repository:**
    ```bash
    git clone https://github.com/DevsHero/db2vec.git 
    cd db2vec
    ```
2.  **Ensure Prerequisites:** Start Ollama and your target vector database. Pull the necessary Ollama model.
3.  **(Optional) Create `.env`:** Create and customize your `.env` file.
4.  **Build:**
    ```bash
    cargo build --release
    ```
5.  **Run:** (See Usage section for detailed arguments)
    ```bash
    # Example: Process a MySQL dump (.sql) and insert into Milvus with auth & debug
    ./target/release/db2vec \
        -f ./mysql_sample.sql \
        -t milvus \
        --host http://127.0.0.1:19530 \
        --database mydb \
        --collection mycol \
        --dimension 768 \
        -u root \
        -p Milvus \
        --use-auth \
        --debug

    # Example: Process SurrealDB (.surql) into Redis (using .env for Ollama)
     ./target/release/db2vec -f ./data/export.surql -t redis --host redis://127.0.0.1:6379
    ```

---

## Usage

```bash
# Using cargo
cargo run -- [OPTIONS]

# Using the compiled binary
./target/release/db2vec [OPTIONS]
```

**Command-Line Options:**

*   `-f, --data-file <DATA_FILE>`: Path to the `.sql` or `.surql` database dump file. [default: `./surreal.surql`]
*   `-t, --db-export-type <DB_EXPORT_TYPE>`: Target vector database (`redis`, `chroma`, `milvus`, `qdrant`, `surreal`). [default: `redis`]
*   `--host <HOST>`: Host URL for the target vector database (e.g., `redis://...`, `http://...`). [env: `HOST`, default: `redis://127.0.0.1:6379`]
*   `--database <DATABASE>`: Target database name (Used by: Chroma, SurrealDB). [env: `DATABASE`, default: `default_database`]
*   `--collection <COLLECTION>`: Target collection/index name (Used by: Milvus, Qdrant, Chroma). [env: `COLLECTION`, default: `my_collection`]
*   `--dimension <DIMENSION>`: Vector dimension size (Used by: Milvus, Qdrant, Chroma). [env: `DIMENSION`, default: `768`]
*   `--debug`: Enable debug mode to print parsed records. [default: `false`]

**Authentication Options:**

*   `--use-auth`: Enable authentication for the target database. [default: `false`]
*   `-u, --user <USER>`: Username for authentication (Used by: Milvus, SurrealDB). [default: `root`]
*   `-p, --pass <PASS>`: Password for authentication (Used by: Milvus, SurrealDB, Redis). [default: ``]
*   `-k, --secret <SECRET>`: Secret key or API token (Used by: Chroma, Qdrant). [default: ``]
*   `--tenant <TENANT>`: Tenant name (Used by: Chroma). [env: `TENANT`, default: `default_tenant`]
*   `--namespace <NAMESPACE>`: Namespace (Used by: SurrealDB). [env: `NAMESPACE`, default: `default_namespace`]

*   `-h, --help`: Print help information.
*   `-V, --version`: Print version information.

---

## Logging & Debugging

`db2vec` uses two mechanisms for providing more detailed output:

1.  **General Logging (`RUST_LOG`):**
    *   Controls the verbosity of internal status messages using the standard Rust `log` crate.
    *   By default, logging is turned **off** (`off` level).
    *   Set the `RUST_LOG` environment variable to `error`, `warn`, `info`, `debug`, or `trace` to see progressively more detail about internal operations (parsing steps, embedding calls, database interactions, etc.).

    ```bash
    # Example: Show informational messages
    RUST_LOG=info ./target/release/db2vec [OPTIONS]

    # Example: Show detailed debug messages from the log crate
    RUST_LOG=debug ./target/release/db2vec [OPTIONS]
    ```

2.  **Parsed Record Output (`--debug` flag):**
    *   A specific command-line flag `--debug` enables printing the fully parsed JSON record structure to standard output *before* it's sent for embedding.
    *   This is useful for verifying the parser's output for specific records.
    *   This flag operates independently of `RUST_LOG`.

    ```bash
    # Example: Print parsed records to stdout (general logging remains off by default)
    ./target/release/db2vec --debug [OPTIONS]
    ```

**Combining Both:**

You can use both `RUST_LOG` and `--debug` simultaneously to get maximum insight:

```bash
# Example: Enable detailed internal logging AND print parsed records
RUST_LOG=debug ./target/release/db2vec --debug [OPTIONS]
```

---

## How It Works

1.  **Read & Detect:** Reads the specified `--data-file` (`.sql` or `.surql`) and automatically detects the specific SQL dialect (MySQL, PostgreSQL, Oracle) or SurrealDB format.
2.  **Parse (Regex):** Efficiently parses the file content using format-specific regular expressions. This process extracts records while handling various value types like strings, numbers, arrays, and JSON objects present in the dump.
3.  **Embed (Ollama):** For each record, generates a vector embedding by sending its textual representation (as JSON) to the configured Ollama model (`EMBEDDING_MODEL`).
4.  **Store (Vector DB):** Connects to the target vector database (`--db-export-type`, `--host`) and inserts each record, typically including a unique ID, the generated vector, and the original parsed data (including complex types) as metadata.

---

## Target Environment

This tool is primarily developed and tested against vector databases running in **Docker containers**, interacting via their **RESTful APIs**. Ensure your target database is accessible from where you run `db2vec`.

---

## Contributing

Contributions are welcome! If you find a bug, have a feature request, or want to improve the parsing or database support, please feel free to open an issue or submit a pull request.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details (or add one if missing).