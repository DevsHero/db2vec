use reqwest::blocking::Client;
use serde_json::{ Value, json };
use log::{ info, warn, error };
use super::{ Database, DbError };
pub struct Args {
    pub host: String,
    pub index: String,
    pub dimension: usize,
    pub metric: Option<String>,
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
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
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

        if
            let Ok(existing) = list_req
                .send()
                .map_err(|e| Box::new(e) as DbError)
                .and_then(|r| r.json::<Vec<String>>().map_err(|e| Box::new(e) as DbError))
        {
            if existing.contains(&pd.index) {
                let desc_url = format!("{}/indexes/{}", pd.host, pd.index);
                let mut desc_req = pd.client.get(&desc_url);
                if !pd.use_auth {
                    desc_req = desc_req
                        .header("Api-Key", pd.api_key.as_ref().unwrap())
                        .header("X-Pinecone-API-Version", &pd.api_version);
                }

                if
                    let Ok(info) = desc_req
                        .send()
                        .map_err(|e| Box::new(e) as DbError)
                        .and_then(|r|
                            r.json::<serde_json::Value>().map_err(|e| Box::new(e) as DbError)
                        )
                {
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
                            let _ = del_req.send().map_err(|e| Box::new(e) as DbError);
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
                .header(
                    "Api-Key",
                    pd.api_key
                        .as_ref()
                        .ok_or_else(|| -> DbError { "API key required for cloud".into() })?
                )
                .header("X-Pinecone-API-Version", &pd.api_version);
        }

        let resp = req
            .json(&payload)
            .send()
            .map_err(|e| Box::new(e) as DbError)?;

        if resp.status().is_success() {
            info!("Created Pinecone index `{}`", pd.index);
        } else {
            warn!(
                "Index `{}` creation responded {}: {}",
                pd.index,
                resp.status(),
                resp.text().map_err(|e| Box::new(e) as DbError)?
            );
        }

        Ok(pd)
    }
}
impl Database for PineconeDatabase {
    fn connect(_url: &str) -> Result<Self, DbError> where Self: Sized {
        unimplemented!("Use PineconeDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        table: &str,
        items: &[(String, Vec<f32>, Value)]
    ) -> Result<(), DbError> {
        if items.is_empty() {
            return Ok(());
        }

        let url = format!("{}/vectors/upsert", self.host);
        let vectors: Vec<Value> = items
            .iter()
            .map(|(id, vector, data)| {
                let vec_values = if vector.is_empty() {
                    warn!("ID='{}': Empty vector received, filling with zeros", id);
                    vec![0.0f32; 768]
                } else {
                    vector.clone()
                };

                let mut record =
                    json!({
                    "id": format!("{}:{}", table, id),
                    "values": vec_values 
                });

                let mut processed_metadata = json!({});
                if let Some(map) = data.as_object() {
                    for (k, v) in map {
                        if v.is_object() || v.is_array() {
                            processed_metadata[k] = Value::String(
                                serde_json::to_string(v).unwrap_or_default()
                            );
                        } else {
                            processed_metadata[k] = v.clone();
                        }
                    }
                }
                record["metadata"] = processed_metadata;
                record
            })
            .collect();

        let payload =
            json!({
            "vectors": vectors,
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
                .and_then(|c| c.as_u64())
                .unwrap_or(0);
            info!("Pinecone: upserted {} vectors into `{}`", count, self.index);
            Ok(())
        } else {
            let status = resp.status();
            let txt = resp.text()?;
            error!("Pinecone bulk upsert failed ({}): {}", status, txt);
            Err(format!("Pinecone bulk upsert error: {}", txt).into())
        }
    }
}
