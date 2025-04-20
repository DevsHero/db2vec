use log::{ info, warn };
use reqwest::blocking::Client;
use serde_json::{ json, Value };
use std::env;
use super::{ Database, DbError };

pub struct QdrantDatabase {
    client: Client,
    url: String,
    _collection_name: String,
    api_key: Option<String>,
}

impl QdrantDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let qdrant_url = args.host.clone();
        let _collection_name = args.collection.clone();
        let dimension = args.dimension;
        let api_key = if args.use_auth && !args.secret.is_empty() {
            Some(args.secret.clone())
        } else {
            None
        };

        let client = Client::new();
        let collection_url = format!("{}/collections/{}", qdrant_url, _collection_name);
        let mut req = client.get(&collection_url);

        if let Some(ref key) = api_key {
            req = req.header("api-key", key);
        }

        let response = req.send().map_err(|e| Box::new(e) as DbError)?;

        if response.status().as_u16() == 404 {
            let create_body =
                json!({
                "vectors": {
                    "size": dimension,
                    "distance": "Cosine" 
                }
            });

            let mut create_req = client.put(&collection_url).json(&create_body);
            if let Some(ref key) = api_key {
                create_req = create_req.header("api-key", key);
            }

            let create_resp = create_req.send().map_err(|e| Box::new(e) as DbError)?;

            if !create_resp.status().is_success() {
                let status = create_resp.status();
                let error_text = create_resp.text().map_err(|e| Box::new(e) as DbError)?;

                return Err(
                    format!(
                        "Failed to create collection: Status: {}, Body: {}",
                        status,
                        error_text
                    ).into()
                );
            }

            info!("Created Qdrant collection: {}", _collection_name);
        } else if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().map_err(|e| Box::new(e) as DbError)?;

            return Err(
                format!(
                    "Failed to check collection: Status: {}, Body: {}",
                    status,
                    error_text
                ).into()
            );
        } else {
            warn!("Qdrant collection already exists: {}", _collection_name);
        }

        Ok(QdrantDatabase { client, url: qdrant_url, _collection_name, api_key })
    }
}

impl Database for QdrantDatabase {
    fn connect(url: &str) -> Result<Self, DbError> where Self: Sized {
        let _collection_name = env
            ::var("QDRANT_COLLECTION")
            .unwrap_or_else(|_| "my_collection".to_string());
        let client = Client::new();

        let api_key = env::var("QDRANT_API_KEY").ok();
        Ok(QdrantDatabase { client, url: url.to_string(), _collection_name, api_key })
    }

    fn store_vector(
        &self,
        table: &str,
        items: &[(String, Vec<f32>, Value)]
    ) -> Result<(), DbError> {
        if items.is_empty() {
            return Ok(());
        }

        let collection_url = format!("{}/collections/{}", self.url, table);
        let mut check_req = self.client.get(&collection_url);
        if let Some(ref key) = self.api_key {
            check_req = check_req.header("api-key", key);
        }

        let response = check_req.send()?;
        if response.status().as_u16() == 404 {
            let dimension = if !items.is_empty() { items[0].1.len() } else { 768 };
            info!("Creating Qdrant collection '{}' with dimension {}", table, dimension);
            let create_body =
                json!({
                "vectors": {
                    "size": dimension,
                    "distance": "Cosine" 
                }
            });

            let mut create_req = self.client.put(&collection_url).json(&create_body);
            if let Some(ref key) = self.api_key {
                create_req = create_req.header("api-key", key);
            }

            let create_resp = create_req.send()?;
            if !create_resp.status().is_success() {
                let error_text = create_resp.text()?;
                return Err(
                    format!("Failed to create collection '{}': {}", table, error_text).into()
                );
            }

            info!("Created Qdrant collection: {}", table);
        }

        let points: Vec<Value> = items
            .iter()
            .map(|(id, vector, payload)| {
                json!({
                "id": id,
                "vector": vector,
                "payload": payload
            })
            })
            .collect();

        let payload = json!({ "points": points });
        let url = format!("{}/collections/{}/points?wait=true", self.url, table);

        let mut req = self.client.put(&url).json(&payload);
        if let Some(ref key) = self.api_key {
            req = req.header("api-key", key);
        }

        let resp = req.send()?;
        if resp.status().is_success() {
            info!("Qdrant: upserted {} points into `{}`", items.len(), table);
            Ok(())
        } else {
            let text = resp.text()?;
            Err(format!("Qdrant upsert failed: {}", text).into())
        }
    }
}
