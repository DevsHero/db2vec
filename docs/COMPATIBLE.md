# Compatibility Matrix

## Supported Vector Database Versions

| Vector DB    | API Version                         | Notes                         |
|--------------|-------------------------------------|-------------------------------|
| Pinecone     | 2025-01                             | Pinecone Cloud Control Plane  |
| Milvus       | v2                                  | Milvus Server API v2          |
| Chroma       | v2                                  | Chroma HTTP API v2            |
| Qdrant       | v1.14.0                             | Qdrant Server v1.14.0         |
| Redis Stack  | redis-stack:7.4.0-v3 (as of 30/4/2025) | Includes RedisJSON, RediSearch |
| SurrealDB    | v2.3.0 (as of 30/4/2025)            | SurrealDB HTTP API v2.3.0     |

---

## Supported Import File Formats

All sample dumps use the latest database‐specific dump format as of 30/4/2025.

| Format       | Sample File             | Notes                           |
|--------------|-------------------------|---------------------------------|
| MSSQL        | `mssql_sample.sql`      | SQLCMD export with `SET NOCOUNT ON` |
| MySQL        | `mysql_sample.sql`      | mysqldump / standard SQL dump   |
| Oracle       | `oracle_sample.sql`     | SQL Developer / expdp format    |
| PostgreSQL   | `postgres_sample.sql`   | `pg_dump --format=plain`        |
| SQLite       | `sqlite_sample.sql`     | `sqlite3 .dump`                 |
| SurrealDB    | `surreal_sample.surql`  | SurrealDB `.surql` export       |

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

## Other Cloud-Hosted Vector Services (Untested)

While we haven’t explicitly tested against managed cloud offerings beyond Pinecone, the same HTTP/API-key patterns should apply:

- **Milvus Cloud** / Zilliz Cloud  
- **Qdrant Cloud**  
- **Redis Enterprise Cloud**  
- **Surreal Cloud**  

To try one of these services:

1.  Set `--host` to your service’s HTTP endpoint.  
2.  Pass your API key or token via `--secret` and enable `--use-auth`.  
3.  Configure any provider-specific flags (e.g. `--indexes`, `--namespace`, etc.).  

db2vec uses standard REST calls and bearer-token auth under the hood, so you may find these services work out-of-the-box. Actual support may vary based on each provider’s API quirks.

