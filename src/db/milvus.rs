use reqwest::blocking::Client;
use serde_json::{ Value, json };
use super::{ Database, DbError };
use log::{ info, warn, error };

pub struct MilvusDatabase {
    url: String,
    _collection_name: String,
    _dimension: usize,
    token: Option<String>,
    client: Client,
}

impl MilvusDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
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

        let resp = req.send().map_err(|e| Box::new(e) as DbError)?;
        let resp_json: serde_json::Value = resp.json().map_err(|e| Box::new(e) as DbError)?;

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

            let create_resp = create_req.send().map_err(|e| Box::new(e) as DbError)?;
            let status = create_resp.status();
            let text = create_resp.text().map_err(|e| Box::new(e) as DbError)?;

            info!("Milvus create collection response: {}", text);
            if !status.is_success() {
                return Err(format!("Failed to create Milvus collection: {}", text).into());
            }
        } else {
            warn!("Collection already exists: {}", _collection_name);
        }

        Ok(MilvusDatabase { url, _collection_name, _dimension, token, client })
    }
}

impl Database for MilvusDatabase {
    fn connect(_url: &str) -> Result<Self, DbError> where Self: Sized {
        unimplemented!("Use MilvusDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        _table: &str,
        items: &[(String, Vec<f32>, Value)]
    ) -> Result<(), DbError> {
        if items.is_empty() {
            return Ok(());
        }

        let data: Vec<Value> = items
            .iter()
            .map(|(id, vec, meta)| {
                let mut rec =
                    json!({
                    "id": id,
                    "vector": vec
                });
                if let Some(obj) = meta.as_object() {
                    for (k, v) in obj {
                        if k != "id" && k != "vector" {
                            rec[k] = v.clone();
                        }
                    }
                }
                rec
            })
            .collect();

        let payload =
            json!({
            "collectionName": self._collection_name,
            "data": data
        });

        let url = format!("{}/v2/vectordb/entities/insert", self.url);
        let mut req = self.client.post(&url).json(&payload);
        if let Some(ref t) = self.token {
            req = req.bearer_auth(t);
        }

        let resp = req.send()?;
        if !resp.status().is_success() {
            let txt = resp.text()?;
            error!("Milvus batch insert failed: {}", txt);
            return Err(format!("Milvus batch insert failed: {}", txt).into());
        }

        info!("Milvus: inserted {} vectors", items.len());
        Ok(())
    }
}
