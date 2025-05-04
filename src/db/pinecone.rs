use reqwest::blocking::Client;
use serde_json::{ Value, json };
use log::{ info, warn, error };
use super::{ Database, DbError };

pub struct PineconeDatabase {
    control_plane_url: String,
    data_plane_url: String,
    client: Client,
    api_version: String,
    api_key: Option<String>,
    use_auth: bool,
    dimension: usize,
}

impl PineconeDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let client = Client::new();
        let api_version = "2025-01".to_string();
        let is_local =
            args.vector_host.contains("localhost") ||
            args.vector_host.contains("127.0.0.1") ||
            args.vector_host.contains("::1");

        let control_plane_url = if is_local {
            args.vector_host.clone()
        } else {
            "https://api.pinecone.io".to_string()
        };

        let mut parsed_host_from_create: Option<String> = None;

        if !args.indexes.is_empty() && !is_local {
            let index_name = args.indexes.as_str();
            let endpoint = "indexes";
            let url = format!("{}/{}", control_plane_url, endpoint);

            let spec = json!({ "serverless": { "cloud": args.cloud, "region": args.region } });
            let body =
                json!({ "name": index_name, "dimension": args.dimension, "metric": args.metric, "spec": spec });

            let mut req = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Pinecone-API-Version", &api_version)
                .json(&body);

            if args.secret.is_empty() {
                return Err("Pinecone cloud requires an API key (-k/--secret).".into());
            }
            req = req.header("Api-Key", &args.secret);

            let resp = req.send()?;
            match resp.status().as_u16() {
                201 | 200 => {
                    let j: Value = resp.json()?;
                    let host = j
                        .get("host")
                        .and_then(|h| h.as_str())
                        .ok_or_else(|| DbError::from("Missing host in create index response"))?;
                    info!("Index '{}' available at host: {}", index_name, host);
                    parsed_host_from_create = Some(format!("https://{}", host));
                }
                409 => {
                    warn!("Index '{}' already exists, attempting to describe it to get host.", index_name);
                    let describe_url = format!("{}/{}", url, index_name);
                    let describe_req = client
                        .get(&describe_url)
                        .header("Accept", "application/json")
                        .header("X-Pinecone-API-Version", &api_version)
                        .header("Api-Key", &args.secret);

                    let describe_resp = describe_req.send()?;
                    if describe_resp.status().is_success() {
                        let j: Value = describe_resp.json()?;
                        let host = j
                            .get("host")
                            .and_then(|h| h.as_str())
                            .ok_or_else(||
                                DbError::from("Missing host in describe index response")
                            )?;
                        info!("Existing index '{}' found at host: {}", index_name, host);
                        parsed_host_from_create = Some(format!("https://{}", host));
                    } else {
                        let txt = describe_resp.text().unwrap_or_default();
                        return Err(
                            format!(
                                "Failed to describe existing index '{}': {}",
                                index_name,
                                txt
                            ).into()
                        );
                    }
                }
                status => {
                    let txt = resp.text().unwrap_or_default();
                    return Err(
                        format!("Failed to create/ensure index ({}): {}", status, txt).into()
                    );
                }
            }
        } else if !args.indexes.is_empty() && is_local {
            warn!(
                "Running locally. Assuming database '{}' exists. Skipping creation/check.",
                args.indexes
            );
        }

        let data_plane_url = if is_local {
            args.vector_host.clone()
        } else {
            if args.vector_host.contains(".svc.") && args.vector_host.contains(".pinecone.io") {
                info!("Using provided --host as data plane URL: {}", args.vector_host);
                if args.vector_host.starts_with("https://") {
                    args.vector_host.clone()
                } else {
                    format!("https://{}", args.vector_host)
                }
            } else if let Some(host) = parsed_host_from_create {
                info!("Using host from create/describe API response as data plane URL: {}", host);
                host
            } else {
                return Err(
                    "Could not determine Pinecone data plane URL. Provide it via --host or ensure --indexes is set correctly.".into()
                );
            }
        };
        if !is_local && args.secret.is_empty() {
            return Err("Pinecone cloud requires an API key (-k/--secret).".into());
        }

        let pd = PineconeDatabase {
            control_plane_url,
            data_plane_url,
            client,
            api_version,
            api_key: if args.secret.is_empty() {
                None
            } else {
                Some(args.secret.clone())
            },
            use_auth: !is_local,
            dimension: args.dimension,
        };

        info!("Pinecone mode: {}", if is_local { "LOCAL" } else { "CLOUD" });
        info!("Control plane URL: {}", pd.control_plane_url);
        info!("Data plane URL: {}", pd.data_plane_url);

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

        let url = format!("{}/vectors/upsert", self.data_plane_url);
        let vectors: Vec<Value> = items
            .iter()
            .map(|(id, vector, data)| {
                let values = if vector.is_empty() {
                    warn!("ID='{}': Empty vector received, inserting dummy values", id);
                    vec![0.1; self.dimension]
                } else if vector.len() != self.dimension {
                    warn!(
                        "ID='{}': Vector length {} != expected dimension {}, fixing",
                        id,
                        vector.len(),
                        self.dimension
                    );
                    let mut fixed_vec = vec![0.0; self.dimension];
                    for (i, val) in vector.iter().enumerate().take(self.dimension) {
                        fixed_vec[i] = *val;
                    }
                    fixed_vec
                } else {
                    vector.clone()
                };

                let mut record =
                    json!({
                    "id": id, 
                    "values": values
                });

                let mut processed_metadata = serde_json::Map::new();
                processed_metadata.insert("table".to_string(), Value::String(table.to_string()));

                if let Some(map) = data.as_object() {
                    for (k, v) in map {
                        if v.is_null() {
                            continue;
                        }
                        if v.is_object() || v.is_array() {
                            processed_metadata.insert(
                                k.clone(),
                                Value::String(serde_json::to_string(v).unwrap_or_default())
                            );
                        } else {
                            processed_metadata.insert(k.clone(), v.clone());
                        }
                    }
                }
                record["metadata"] = Value::Object(processed_metadata);
                record
            })
            .collect();

        let payload =
            json!({
            "vectors": vectors,
            "namespace": table 
        });

        let mut req = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("X-Pinecone-API-Version", &self.api_version);

        if self.use_auth {
            if let Some(key) = self.api_key.as_ref() {
                req = req.header("Api-Key", key);
            } else {
                error!("Pinecone auth enabled but no API key available.");
                return Err("Pinecone auth enabled but no API key available.".into());
            }
        }

        let resp = req.json(&payload).send()?;
        if resp.status().is_success() {
            let j: Value = resp.json()?;
            let count = j
                .get("upsertedCount")
                .and_then(|c| c.as_u64())
                .unwrap_or(0);
            info!("Pinecone: upserted {} vectors into namespace `{}`", count, table);
            Ok(())
        } else {
            let status = resp.status();
            let txt = resp.text()?;
            error!("Pinecone bulk upsert failed for namespace '{}' ({}): {}", table, status, txt);
            Err(format!("Pinecone bulk upsert error for namespace '{}': {}", table, txt).into())
        }
    }
}
