use log::{ info, warn, debug };
use reqwest::blocking::Client;
use serde_json::Value;
use super::{ Database, DbError };

pub struct ChromaDatabase {
    client: Client,
    url: String,
    tenant: String,
    database: String,
    dimension: usize,
    auth_token: Option<String>,
    metric: String,
}

impl ChromaDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let url = format!("{}/api/v2", args.host.trim_end_matches('/'));
        let tenant = args.tenant.clone();
        let database = args.database.clone();
        let dimension = args.dimension;
        let client = Client::new();
        let auth_token = if args.use_auth && !args.secret.is_empty() {
            Some(args.secret.clone())
        } else {
            None
        };

        let metric = args.metric.clone();
        Ok(ChromaDatabase {
            client,
            url,
            tenant,
            database,
            dimension,
            auth_token,
            metric,
        })
    }
}

impl Database for ChromaDatabase {
    fn connect(_url: &str) -> Result<Self, DbError> where Self: Sized {
        unimplemented!("Use ChromaDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        table: &str,
        items: &[(String, Vec<f32>, Value)]
    ) -> Result<(), DbError> {
        if items.is_empty() {
            return Ok(());
        }

        let dbs_url = format!("{}/tenants/{}/databases", self.url, self.tenant);
        let mut list_dbs_req = self.client.get(&dbs_url);
        if let Some(ref token) = self.auth_token {
            list_dbs_req = list_dbs_req.header("X-Chroma-Token", token);
        }
        let dbs_json: Value = list_dbs_req.send()?.json()?;
        let db_exists = dbs_json
            .as_array()
            .map(|arr| arr.iter().any(|db| db["name"].as_str() == Some(&self.database)))
            .unwrap_or(false);
        if !db_exists {
            info!("Creating Chroma database '{}'", self.database);
            let mut create_db_req = self.client
                .post(&dbs_url)
                .json(&serde_json::json!({ "name": self.database }));
            if let Some(ref token) = self.auth_token {
                create_db_req = create_db_req.header("X-Chroma-Token", token);
            }
            let create_db_res = create_db_req.send()?;
            if !create_db_res.status().is_success() {
                let err = create_db_res.text()?;
                return Err(
                    format!("Failed to create Chroma database '{}': {}", self.database, err).into()
                );
            }
            info!("Chroma database '{}' created", self.database);
        }

        let collections_url = format!(
            "{}/tenants/{}/databases/{}/collections",
            self.url,
            self.tenant,
            self.database
        );
        let mut list_req = self.client.get(&collections_url);
        if let Some(ref token) = self.auth_token {
            list_req = list_req.header("X-Chroma-Token", token);
        }
        let cols_res = list_req.send()?;
        let cols_json: Value = cols_res.json()?;
        let mut collection_id: Option<String> = None;
        if let Some(arr) = cols_json.as_array() {
            for col in arr {
                if col["name"].as_str() == Some(table) {
                    collection_id = col["id"].as_str().map(|s| s.to_string());
                    break;
                }
            }
        }
        let collection_id = match collection_id {
            Some(id) => id,
            None => {
                let col_body =
                    serde_json::json!({
                    "name": table,
                    "dimension": self.dimension,
                    "configuration_json": {
                        "embedding_function": null,
                        "hnsw": {
                            "space": self.metric,  
                            "ef_construction": 100,
                            "ef_search": 100,
                            "max_neighbors": 16,
                            "resize_factor": 1.2,
                            "sync_threshold": 1000
                        },
                        "spann": null
                    }
                });
                let mut col_req = self.client.post(&collections_url).json(&col_body);
                if let Some(ref token) = self.auth_token {
                    col_req = col_req.header("X-Chroma-Token", token);
                }
                let col_res = col_req.send()?;
                let col_json: Value = col_res.json()?;
                debug!("Chroma create collection response: {}", col_json);

                col_json
                    .get("id")
                    .or_else(|| col_json.get("collection_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        format!("Failed to get new collection id, response: {}", col_json)
                    })?
                    .to_string()
            }
        };

        let ids: Vec<String> = items
            .iter()
            .map(|(id, _, _)| format!("{}:{}", table, id))
            .collect();
        let embeddings: Vec<Vec<f32>> = items
            .iter()
            .map(|(id, vec, _)| {
                if vec.is_empty() {
                    warn!("ID='{}': Empty vector received, inserting dummy value", id);
                    vec![0.1]
                } else if vec.len() != self.dimension {
                    warn!(
                        "ID='{}': Vector length {} != collection dimension {}, fixing",
                        id,
                        vec.len(),
                        self.dimension
                    );
                    let mut fixed_vec = vec![0.0; self.dimension];
                    for (i, val) in vec.iter().enumerate().take(self.dimension) {
                        fixed_vec[i] = *val;
                    }
                    fixed_vec
                } else {
                    vec.clone()
                }
            })
            .collect();
        let documents: Vec<String> = items
            .iter()
            .map(|_| String::new())
            .collect();
        let metadatas: Vec<Value> = items
            .iter()
            .map(|(_, _, m)| {
                if let Some(map) = m.as_object() {
                    let mut simple = serde_json::Map::new();
                    for (k, v) in map {
                        if v.is_string() || v.is_number() || v.is_boolean() {
                            simple.insert(k.clone(), v.clone());
                        }
                    }
                    if simple.is_empty() {
                        Value::Null
                    } else {
                        Value::Object(simple)
                    }
                } else {
                    Value::Null
                }
            })
            .collect();

        let body =
            serde_json::json!({
            "ids": ids,
            "embeddings": embeddings,
            "documents": documents,
            "metadatas": metadatas
        });

        let add_url = format!(
            "{}/tenants/{}/databases/{}/collections/{}/add",
            self.url,
            self.tenant,
            self.database,
            collection_id
        );
        let mut req = self.client.post(&add_url).json(&body);
        if let Some(ref token) = self.auth_token {
            req = req.header("X-Chroma-Token", token);
        }
        let resp = req.send()?;

        let status = resp.status();
        let body_text = resp.text()?;
        debug!("Chroma insert response ({}): {}", status, body_text);

        if status.is_success() {
            info!("Chroma: inserted {} vectors into '{}'", items.len(), table);
            Ok(())
        } else if body_text.contains("Error in compaction") {
            warn!("Chroma compaction error during insert (ignored): {}", body_text);
            info!(
                "Chroma: potentially inserted {} vectors into '{}' despite compaction error",
                items.len(),
                table
            );
            Ok(())
        } else {
            Err(format!("Chroma bulk insert failed: {}", body_text).into())
        }
    }
}
