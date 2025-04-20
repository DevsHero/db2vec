use log::{ info, warn };
use reqwest::blocking::Client;
use serde_json::Value;
use super::{ Database, DbError };

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
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
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

        let tenant_resp = tenant_req.send().map_err(|e| Box::new(e) as DbError)?;

        if !tenant_resp.status().is_success() && tenant_resp.status().as_u16() != 409 {
            let err = tenant_resp.text().map_err(|e| Box::new(e) as DbError)?;
            return Err(format!("Failed to create tenant: {}", err).into());
        }

        let databases_url = format!("{}/tenants/{}/databases", url, tenant);
        let db_body = serde_json::json!({ "name": database });
        let mut db_req = client.post(&databases_url).json(&db_body);

        if let Some(ref token) = auth_token {
            db_req = db_req.header("X-Chroma-Token", token);
        }
        let db_resp = db_req.send().map_err(|e| Box::new(e) as DbError)?;

        if !db_resp.status().is_success() && db_resp.status().as_u16() != 409 {
            let err = db_resp.text().map_err(|e| Box::new(e) as DbError)?;
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
        let col_resp = col_req.send().map_err(|e| Box::new(e) as DbError)?;
        if col_resp.status().is_success() {
            info!("Collection created: {}", _collection_name);
        } else if col_resp.status().as_u16() == 409 {
            warn!("Collection already exists: {}", _collection_name);
        } else {
            let err = col_resp.text().map_err(|e| Box::new(e) as DbError)?;
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
        let resp = list_req.send().map_err(|e| Box::new(e) as DbError)?;
        let collections: Value = resp.json().map_err(|e| Box::new(e) as DbError)?;
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
}

impl Database for ChromaDatabase {
    fn connect(_url: &str) -> Result<Self, DbError> where Self: Sized {
        unimplemented!("Use ChromaDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        _table: &str,
        items: &[(String, Vec<f32>, Value)]
    ) -> Result<(), DbError> {
        if items.is_empty() {
            return Ok(());
        }

        let ids: Vec<String> = items
            .iter()
            .map(|(id, _, _)| format!("{}:{}", _table, id))
            .collect();

        let embeddings: Vec<Vec<f32>> = items
            .iter()
            .map(|(_, v, _)| v.clone())
            .collect();

        let documents: Vec<String> = items
            .iter()
            .map(|_| String::new())
            .collect();

        let metadatas: Vec<Value> = items
            .iter()
            .map(|(_, _, meta)| {
                if let Some(obj) = meta.as_object() {
                    let mut simple_meta = serde_json::Map::new();
                    for (k, v) in obj {
                        if v.is_string() || v.is_number() || v.is_boolean() {
                            simple_meta.insert(k.clone(), v.clone());
                        }
                    }

                    if simple_meta.is_empty() {
                        Value::Null
                    } else {
                        Value::Object(simple_meta)
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
            self.collection_id
        );

        let mut req = self.client.post(&add_url).json(&body);
        if let Some(ref token) = self.auth_token {
            req = req.header("X-Chroma-Token", token);
        }

        let resp = req.send().map_err(|e| Box::new(e) as DbError)?;
        if resp.status().is_success() {
            info!("Chroma: inserted {} vectors", items.len());
            Ok(())
        } else {
            let text = resp.text().map_err(|e| Box::new(e) as DbError)?;
            Err(format!("Chroma bulk insert failed: {}", text).into())
        }
    }
}
