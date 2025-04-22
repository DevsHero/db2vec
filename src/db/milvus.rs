use reqwest::blocking::Client;
use serde_json::{ Value, json };
use super::{ Database, DbError };
use log::{ debug, error, info, warn };

pub struct MilvusDatabase {
    url: String,
    token: Option<String>,
    client: Client,
    dimension: usize,
    db_name: String,
}

impl MilvusDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let url = args.host.clone();
        let db_name = args.database.clone();
        let token = if args.use_auth && (!args.user.is_empty() || !args.pass.is_empty()) {
            Some(format!("{}:{}", args.user, args.pass))
        } else {
            None
        };
        let client = Client::new();

        Ok(MilvusDatabase { url, token, client, dimension: args.dimension, db_name })
    }
}

impl Database for MilvusDatabase {
    fn connect(_url: &str) -> Result<Self, DbError> where Self: Sized {
        unimplemented!("Use MilvusDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        table: &str,
        items: &[(String, Vec<f32>, Value)]
    ) -> Result<(), DbError> {
        if items.is_empty() {
            return Ok(());
        }

        let stats_url = format!("{}/v2/vectordb/collections/get_stats", self.url);
        let mut stats_req = self.client
            .post(&stats_url)
            .json(
                &json!({
                "dbName": self.db_name,
                "collectionName": table
            })
            );
        if let Some(ref t) = self.token {
            stats_req = stats_req.bearer_auth(t);
        }
        let stats = stats_req.send().map_err(|e| Box::new(e) as DbError)?;
        let stats_json: Value = stats.json().map_err(|e| Box::new(e) as DbError)?;
        let exists = stats_json.get("code").and_then(|c| c.as_i64()) == Some(0);

        if !exists {
            info!("Creating Milvus collection '{}'", table);
            let create_url = format!("{}/v2/vectordb/collections/create", self.url);
            let mut create_req = self.client
                .post(&create_url)
                .json(
                    &json!({
                    "dbName": self.db_name,
                    "collectionName": table,
                    "dimension": self.dimension,
                    "primaryFieldName": "id",
                    "idType": "VarChar",
                    "vectorFieldName": "vector",
                    "metric_type": "L2",
                    "autoId": false,
                    "params": { "max_length": "128" }
                })
                );
            if let Some(ref t) = self.token {
                create_req = create_req.bearer_auth(t);
            }
            let resp = create_req.send().map_err(|e| Box::new(e) as DbError)?;
            let status = resp.status();
            let text = resp.text().map_err(|e| Box::new(e) as DbError)?;
            if !status.is_success() {
                return Err(
                    format!("Failed to create Milvus collection '{}': {}", table, text).into()
                );
            }
            info!("Milvus collection '{}' created", table);
        }

        let data: Vec<Value> = items
            .iter()
            .map(|(id, vec, meta)| {
                let v = if vec.len() == self.dimension {
                    vec.clone()
                } else {
                    warn!(
                        "ID='{}': vector length {} ≠ {}, filling with zeros",
                        id,
                        vec.len(),
                        self.dimension
                    );
                    vec![0.0; self.dimension]
                };

                let mut obj = json!({ "id": id, "vector": v });
                if let Some(map) = meta.as_object() {
                    for (k, v) in map.iter() {
                        if k != "id" && k != "vector" {
                            obj[k] = v.clone();
                        }
                    }
                }
                obj
            })
            .collect();

        let insert_url = format!("{}/v2/vectordb/entities/insert", self.url);
        let mut ins_req = self.client
            .post(&insert_url)
            .json(
                &json!({
                "dbName": self.db_name,
                "collectionName": table,
                "data": data
            })
            );
        if let Some(ref t) = self.token {
            ins_req = ins_req.bearer_auth(t);
        }
        let ins_res = ins_req.send()?;
        let status = ins_res.status();
        let resp_text = ins_res.text()?;
        debug!("Milvus insert response ({}): {}", status, resp_text);
        if !status.is_success() {
            error!("Milvus insert failed for '{}': {}", table, resp_text);
            return Err(format!("Milvus insert failed: {}", resp_text).into());
        }
        info!("Milvus: inserted {} vectors into '{}'", items.len(), table);
        let flush_url = format!("{}/v2/vectordb/collections/flush", self.url);
        let mut flush_req = self.client
            .post(&flush_url)
            .json(
                &json!({
                "dbName": self.db_name,
                "collectionName": table
            })
            );
        if let Some(ref t) = self.token {
            flush_req = flush_req.bearer_auth(t);
        }
        let flush_res = flush_req.send()?;
        if !flush_res.status().is_success() {
            let err = flush_res.text()?;
            warn!("Milvus flush failed for '{}': {}", table, err);
        } else {
            info!("Milvus: flushed collection '{}'", table);
        }

        Ok(())
    }
}
