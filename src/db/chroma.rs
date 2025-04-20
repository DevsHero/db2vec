use log::{ info, warn };
use reqwest::blocking::Client;
use serde_json::Value;
use std::error::Error;

use super::Database;

pub struct ChromaDatabase {
    client: Client,
    url: String,
    tenant: String,
    database: String,
    _collection_name: String,
    collection_id: String,
    auth_token: Option<String>,
}

impl ChromaDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, Box<dyn Error>> {
        let url = format!("{}/api/v2", args.host.trim_end_matches('/'));
        let tenant = args.tenant.clone();
        let database = args.database.clone();
        let _collection_name = args.collection.clone();
        let dimension = args.dimension;
        let client = Client::new();
        let auth_token = if args.use_auth && !args.secret.is_empty() {
            Some(args.secret.clone())
        } else {
            None
        };

        let tenants_url = format!("{}/tenants", url);
        let tenant_body = serde_json::json!({ "name": tenant });
        let mut tenant_req = client.post(&tenants_url).json(&tenant_body);
        if let Some(ref token) = auth_token {
            tenant_req = tenant_req.header("X-Chroma-Token", token);
        }
        let tenant_resp = tenant_req.send()?;
        if !tenant_resp.status().is_success() && tenant_resp.status().as_u16() != 409 {
            let err = tenant_resp.text()?;
            return Err(format!("Failed to create tenant: {}", err).into());
        }

        let databases_url = format!("{}/tenants/{}/databases", url, tenant);
        let db_body = serde_json::json!({ "name": database });
        let mut db_req = client.post(&databases_url).json(&db_body);
        if let Some(ref token) = auth_token {
            db_req = db_req.header("X-Chroma-Token", token);
        }
        let db_resp = db_req.send()?;
        if !db_resp.status().is_success() && db_resp.status().as_u16() != 409 {
            let err = db_resp.text()?;
            return Err(format!("Failed to create database: {}", err).into());
        }

        let collections_url = format!(
            "{}/tenants/{}/databases/{}/collections",
            url,
            tenant,
            database
        );
        let col_body = serde_json::json!({ "name": _collection_name, "dimension": dimension });
        let mut col_req = client.post(&collections_url).json(&col_body);
        if let Some(ref token) = auth_token {
            col_req = col_req.header("X-Chroma-Token", token);
        }
        let col_resp = col_req.send()?;
        if col_resp.status().is_success() {
            info!("Collection created: {}", _collection_name);
        } else if col_resp.status().as_u16() == 409 {
            warn!("Collection already exists: {}", _collection_name);
        } else {
            let err = col_resp.text()?;
            return Err(format!("Failed to create collection: {}", err).into());
        }

        let collections_url = format!(
            "{}/tenants/{}/databases/{}/collections",
            url,
            tenant,
            database
        );
        let mut list_req = client.get(&collections_url);
        if let Some(ref token) = auth_token {
            list_req = list_req.header("X-Chroma-Token", token);
        }
        let resp = list_req.send()?;
        let collections: Value = resp.json()?;
        let collection_id = collections
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .find(|c| c["name"] == _collection_name)
            .and_then(|c| c["id"].as_str())
            .ok_or("Collection UUID not found")?
            .to_string();

        Ok(ChromaDatabase {
            client,
            url,
            tenant,
            database,
            _collection_name,
            collection_id,
            auth_token,
        })
    }

    pub fn add_vector(
        &self,
        id: &str,
        vector: &[f32],
        metadata: &Value,
        document: &str
    ) -> Result<(), Box<dyn Error>> {
        let add_url = format!(
            "{}/tenants/{}/databases/{}/collections/{}/add",
            self.url,
            self.tenant,
            self.database,
            self.collection_id
        );
        let body =
            serde_json::json!({
            "ids": [id],
            "embeddings": [vector],
            "documents": [document],
            "metadatas": [metadata],
        });

        let mut add_req = self.client.post(&add_url).json(&body);
        if let Some(ref token) = self.auth_token {
            add_req = add_req.header("X-Chroma-Token", token);
        }
        let resp = add_req.send()?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let err = resp.text()?;
            Err(format!("Failed to add vector to Chroma: {}", err).into())
        }
    }
}

impl Database for ChromaDatabase {
    fn connect(_url: &str) -> Result<Self, Box<dyn Error>> where Self: Sized {
        unimplemented!("Use ChromaDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        _table: &str,
        key: &str,
        vector: &[f32],
        data: &Value
    ) -> Result<(), Box<dyn Error>> {
        let record_text = serde_json::to_string(&data)?;
        self.add_vector(key, vector, &data, &record_text)?;
        Ok(())
    }
}
