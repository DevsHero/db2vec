use reqwest::blocking::Client;
use serde_json::{ Value, json };
use std::error::Error;
use log::{ info, warn, error };
use super::Database;
pub struct Args {
    pub host: String,
    pub index: String,
    pub dimension: usize,
    pub metric: Option<String>, // "cosine", "euclidean", or "dotproduct"
    pub api_key: Option<String>,
    pub use_auth: bool,
    pub namespace: String,
}
pub struct PineconeDatabase {
    host: String,
    index: String,
    client: Client,
    api_version: String,
    api_key: Option<String>,
    use_auth: bool,
    namespace: String,
}

impl PineconeDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, Box<dyn Error>> {
        let client = Client::new();
        let api_version = "2025-04".to_string();
        let pd = PineconeDatabase {
            host: args.host.clone(),
            index: args.collection.clone(),
            namespace: args.namespace.clone(),
            client,
            api_version,
            api_key: Some(args.secret.clone()),
            use_auth: args.use_auth,
        };

        let list_url = format!("{}/indexes", pd.host);
        let mut list_req = pd.client.get(&list_url);
        if !pd.use_auth {
            list_req = list_req
                .header("Api-Key", pd.api_key.as_ref().unwrap())
                .header("X-Pinecone-API-Version", &pd.api_version);
        }
        if let Ok(existing) = list_req.send().and_then(|r| r.json::<Vec<String>>()) {
            if existing.contains(&pd.index) {
                let desc_url = format!("{}/indexes/{}", pd.host, pd.index);
                let mut desc_req = pd.client.get(&desc_url);
                if !pd.use_auth {
                    desc_req = desc_req
                        .header("Api-Key", pd.api_key.as_ref().unwrap())
                        .header("X-Pinecone-API-Version", &pd.api_version);
                }
                if let Ok(info) = desc_req.send().and_then(|r| r.json::<serde_json::Value>()) {
                    if let Some(curr_dim) = info.get("dimension").and_then(|d| d.as_u64()) {
                        if (curr_dim as usize) != args.dimension {
                            warn!(
                                "Index '{}' exists with dimension {} but args.dimension is {}. Deleting.",
                                pd.index,
                                curr_dim,
                                args.dimension
                            );
                            let del_url = format!("{}/indexes/{}", pd.host, pd.index);
                            let mut del_req = pd.client.delete(&del_url);
                            if !pd.use_auth {
                                del_req = del_req
                                    .header("Api-Key", pd.api_key.as_ref().unwrap())
                                    .header("X-Pinecone-API-Version", &pd.api_version);
                            }
                            let _ = del_req.send();
                        }
                    }
                }
            }
        }

        let url = format!("{}/indexes", pd.host);
        let payload =
            json!({
            "name": pd.index,
            "dimension": args.dimension,
            "metric": args.metric,
        });

        let mut req = pd.client.post(&url).header("Content-Type", "application/json");
        if !pd.use_auth {
            req = req
                .header("Api-Key", pd.api_key.as_ref().ok_or("API key required for cloud")?)
                .header("X-Pinecone-API-Version", &pd.api_version);
        }
        let resp = req.json(&payload).send()?;
        if resp.status().is_success() {
            info!("Created Pinecone index `{}`", pd.index);
        } else {
            warn!("Index `{}` creation responded {}: {}", pd.index, resp.status(), resp.text()?);
        }

        Ok(pd)
    }

    pub fn upsert_vector(
        &self,
        id: &str,
        vector: Vec<f32>,
        metadata: Option<Value>
    ) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/vectors/upsert", self.host);
        let mut record = json!({
            "id": id,
            "values": vector
        });
        if let Some(meta) = metadata {
            record["metadata"] = meta;
        }

        let payload =
            json!({
            "vectors": [ record ],
            "namespace": self.namespace
        });

        let mut req = self.client.post(&url).header("Content-Type", "application/json");

        if !self.use_auth {
            req = req
                .header("Api-Key", self.api_key.as_ref().unwrap())
                .header("X-Pinecone-API-Version", &self.api_version);
        }

        let resp = req.json(&payload).send()?;
        if resp.status().is_success() {
            let j: Value = resp.json()?;
            let count = j
                .get("upsertedCount")
                .and_then(|c| c.as_i64())
                .unwrap_or(0);
            info!("Upserted {} vector(s) into `{}`", count, self.index);
            Ok(())
        } else {
            let status = resp.status();
            let txt = resp.text()?;
            error!("Upsert failed ({}): {}", status, txt);
            Err(format!("Upsert error: {}", txt).into())
        }
    }

    pub fn upsert_text(
        &self,
        id: &str,
        chunk_text: &str,
        category: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/records/namespaces/{}/upsert", self.host, self.namespace);

        let rec =
            json!({
            "_id": id,
            "chunk_text": chunk_text,
            "category": category
        });
        let ndjson = serde_json::to_string(&rec)? + "\n";
        let mut req = self.client.post(&url).header("Content-Type", "application/x-ndjson");

        if !self.use_auth {
            req = req
                .header("Api-Key", self.api_key.as_ref().unwrap())
                .header("X-Pinecone-API-Version", &self.api_version);
        }

        let resp = req.body(ndjson).send()?;
        if resp.status().is_success() {
            log::info!("Upserted text record `{}`", id);
            Ok(())
        } else {
            let status = resp.status();
            let txt = resp.text()?;
            log::error!("Text upsert failed ({}): {}", status, txt);
            Err(format!("Text upsert error: {}", txt).into())
        }
    }
}
impl Database for PineconeDatabase {
    fn connect(_url: &str) -> Result<Self, Box<dyn Error>> where Self: Sized {
        unimplemented!("Use PineconeDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        table: &str,
        key: &str,
        vector: &[f32],
        data: &Value
    ) -> Result<(), Box<dyn Error>> {
        let combined_id = format!("{}:{}", table, key);
        let vec = vector.to_vec();
        let mut processed_metadata = json!({});
        if let Some(map) = data.as_object() {
            for (k, v) in map {
                if v.is_object() || v.is_array() {
                    processed_metadata[k] = Value::String(serde_json::to_string(v)?);
                } else {
                    processed_metadata[k] = v.clone();
                }
            }
        }
        let metadata = Some(processed_metadata);
        self.upsert_vector(&combined_id, vec, metadata)
    }
}
