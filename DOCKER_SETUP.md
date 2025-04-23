# Local Vector Database Setup with Docker

This guide provides quick‑start Docker commands for running supported vector databases locally with `db2vec`. For full details and advanced options, please refer to the official documentation links provided for each database.

---

## Pinecone (Local Development)

Official docs: https://docs.pinecone.io/guides/operations/local-development#docker-cli

```bash
docker run -d \
  --name dense-index \
  -e PORT=5081 \
  -e INDEX_TYPE=serverless \
  -e VECTOR_TYPE=dense \
  -e DIMENSION=768 \
  -e METRIC=cosine \
  -p 5081:5081 \
  --platform linux/amd64 \
  ghcr.io/pinecone-io/pinecone-index:latest
```

---

## SurrealDB

Official docs: https://surrealdb.com/docs/surrealdb/installation/running/docker

```bash
docker run -d --rm --pull always \
  -p 8000:8000 \
  -v /mydata:/mydata \
  surrealdb/surrealdb:latest \
  start --user root --pass root
```

---

## Milvus (Standalone)

Official docs: https://milvus.io/docs/configure-docker.md?tab=component

```bash
wget https://github.com/milvus-io/milvus/releases/download/v2.5.9/milvus-standalone-docker-compose.yml \
  -O docker-compose.yml
docker compose up -d
```

---

## Redis Stack

Official docs: https://hub.docker.com/r/redis/redis-stack

```bash
docker run -d \
  --name redis-stack \
  -p 6379:6379 \
  -p 8001:8001 \
  redis/redis-stack:latest
```

---

## Chroma

Official docs: https://docs.trychroma.com/production/containers/docker

```bash
docker run -d \
  -v ./chroma-data:/data \
  -p 8000:8000 \
  chromadb/chroma
```

---

## Qdrant

Official docs: https://qdrant.tech/documentation/quickstart/

```bash
docker run -d \
  -p 6333:6333 \
  -p 6334:6334 \
  -v "$(pwd)/qdrant_storage:/qdrant/storage:z" \
  qdrant/qdrant
```

---

> **Note:** Always consult the official documentation for each database for the latest setup instructions, environment variables, and recommended production configurations.  
>  
> Save this file as `DOCKER_SETUP.md` in your project root and copy the commands as needed.  