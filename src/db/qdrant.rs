use reqwest::blocking::Client;
use serde_json::{ json, Value };
use std::env;
use std::error::Error;

use super::Database;

pub struct QdrantDatabase {
    client: Client,
    url: String,
    collection_name: String,
    api_key: Option<String>,
}

impl QdrantDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, Box<dyn Error>> {
        let qdrant_url = args.host.clone();
        let collection_name = args.collection.clone();
        let dimension = args.dimension;
        let api_key = if args.use_auth && !args.secret.is_empty() {
            Some(args.secret.clone())
        } else {
            None
        };
        let client = Client::new();
        let collection_url = format!("{}/collections/{}", qdrant_url, collection_name);
        let mut req = client.get(&collection_url);
        if let Some(ref key) = api_key {
            req = req.header("api-key", key);
        }
        let response = req.send()?;

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
            let create_resp = create_req.send()?;

            if !create_resp.status().is_success() {
                let status = create_resp.status();
                let error_text = create_resp.text()?;
                return Err(
                    format!(
                        "Failed to create collection: Status: {}, Body: {}",
                        status,
                        error_text
                    ).into()
                );
            }
            println!("Created Qdrant collection: {}", collection_name);
        } else if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text()?;
            return Err(
                format!(
                    "Failed to check collection: Status: {}, Body: {}",
                    status,
                    error_text
                ).into()
            );
        } else {
            println!("Qdrant collection already exists: {}", collection_name);
        }

        Ok(QdrantDatabase { client, url: qdrant_url, collection_name, api_key })
    }

    pub fn upload_vector(
        &self,
        id: &str,
        vector: &[f32],
        metadata: &Value
    ) -> Result<(), Box<dyn Error>> {
        let body =
            json!({
            "points": [
                {
                    "id": id,
                    "vector": vector,
                    "payload": metadata,
                }
            ]
        });

        let points_url = format!(
            "{}/collections/{}/points?wait=true",
            self.url,
            self.collection_name
        );

        let mut req = self.client.put(&points_url).json(&body);
        if let Some(ref key) = self.api_key {
            req = req.header("api-key", key);
        }
        let response = req.send()?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text()?;
            Err(format!("Failed to upload vector: Status: {}, Body: {}", status, error_text).into())
        }
    }
}

impl Database for QdrantDatabase {
    fn connect(url: &str) -> Result<Self, Box<dyn Error>> where Self: Sized {
        let collection_name = env
            ::var("QDRANT_COLLECTION")
            .unwrap_or_else(|_| "my_collection".to_string());
        let client = Client::new();

        let api_key = env::var("QDRANT_API_KEY").ok();
        Ok(QdrantDatabase { client, url: url.to_string(), collection_name, api_key })
    }

    fn store_vector(
        &self,
        _table: &str,
        key: &str,
        vector: &[f32],
        data: &Value
    ) -> Result<(), Box<dyn Error>> {
        self.upload_vector(key, vector, &data)
    }
}
