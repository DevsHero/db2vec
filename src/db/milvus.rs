use reqwest::blocking::Client;
use serde_json::{ Value, json };
use std::error::Error;
use super::Database;
use log::{ info, warn, error };

pub struct MilvusDatabase {
    url: String,
    _collection_name: String,
    _dimension: usize,
    token: Option<String>,
    client: Client,
}

impl MilvusDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, Box<dyn Error>> {
        let url = args.host.clone();
        let _collection_name = args.collection.clone();
        let _dimension = args.dimension;
        let token = if args.use_auth && (!args.user.is_empty() || !args.pass.is_empty()) {
            Some(format!("{}:{}", args.user, args.pass))
        } else {
            None
        };
        let client = Client::new();

        let stats_url = format!("{}/v2/vectordb/collections/get_stats", url);
        let payload = json!({ "collectionName": _collection_name });
        let mut req = client.post(&stats_url).json(&payload);
        if let Some(ref t) = token {
            req = req.bearer_auth(t);
        }
        let resp = req.send()?;
        let resp_json: serde_json::Value = resp.json()?;
        let exists = resp_json.get("code").and_then(|c| c.as_i64()) == Some(0);

        if !exists {
            let create_url = format!("{}/v2/vectordb/collections/create", url);
            let payload =
                json!({
                "collectionName": _collection_name,
                "dimension": _dimension,
                "primaryFieldName": "id",
                "idType": "VarChar",
                "vectorFieldName": "vector",
                "metric_type": "L2",
                "autoId": false,
                "params": {
                    "max_length": "128" 
                }
            });
            let mut create_req = client.post(&create_url).json(&payload);
            if let Some(ref t) = token {
                create_req = create_req.bearer_auth(t);
            }
            let create_resp = create_req.send()?;
            let status = create_resp.status();
            let text = create_resp.text()?;
            info!("Milvus create collection response: {}", text);
            if !status.is_success() {
                return Err(format!("Failed to create Milvus collection: {}", text).into());
            }
        } else {
            warn!("Collection already exists: {}", _collection_name);
        }

        Ok(MilvusDatabase { url, _collection_name, _dimension, token, client })
    }

    pub fn insert_vector_via_rest(
        &self,
        _collection_name: &str,
        id: &str,
        vector: Vec<f32>,
        metadata: Option<Value>
    ) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/v2/vectordb/entities/insert", self.url);
        let mut record =
            json!({
            "id": id, // Use the combined table:key as primary key
            "vector": vector
        });

        if let Some(meta) = metadata {
            if let Some(obj) = meta.as_object() {
                for (k, v) in obj {
                    if k != "id" && k != "vector" {
                        record[k] = v.clone();
                    }
                }
            }
        }

        let payload =
            json!({
            "collectionName": _collection_name,
            "data": [record]
        });

        let mut req = self.client.post(&url).json(&payload);
        // Conditionally add the Authorization header
        if let Some(ref t) = self.token {
            req = req.bearer_auth(t);
        }

        let resp = req.send()?;
        let status = resp.status();
        let text = resp.text()?;
        info!("Milvus insert response: {}", status);
        if !status.is_success() {
            error!("Failed to insert vector (Status: {}): {}", status, text);
            return Err(format!("Failed to insert vector: {}", text).into());
        }
        Ok(())
    }
}

impl Database for MilvusDatabase {
    fn connect(_url: &str) -> Result<Self, Box<dyn Error>> where Self: Sized {
        unimplemented!("Use MilvusDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        table: &str, // Table name is still used for the combined key
        key: &str,
        vector: &[f32],
        data: &Value
    ) -> Result<(), Box<dyn Error>> {
        let vec = vector.to_vec();
        let combined_key = format!("{}:{}", table, key);
        self.insert_vector_via_rest(&self._collection_name, &combined_key, vec, Some(data.clone()))
    }
}
