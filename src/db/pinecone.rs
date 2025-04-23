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
    namespace: String,
    dimension: usize,
}

impl PineconeDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let client = Client::new();
        let api_version = "2025-01".to_string();
        let is_local =
            args.host.contains("localhost") ||
            args.host.contains("127.0.0.1") ||
            args.host.contains("::1");

        let control_plane_url = if is_local {
            args.host.clone()
        } else {
            "https://api.pinecone.io".to_string()
        };

        let data_plane_url = args.host.clone();

        if !is_local && args.secret.is_empty() {
            return Err(
                "Pinecone cloud requires an API key. Use -k/--secret to provide one.".into()
            );
        }

        let pd = PineconeDatabase {
            control_plane_url,
            data_plane_url,
            namespace: args.namespace.clone(),
            client,
            api_version,
            api_key: Some(args.secret.clone()),
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
                    "id": format!("{}:{}", table, id),
                    "values": values
                });

                let mut processed_metadata = json!({});
                if let Some(map) = data.as_object() {
                    for (k, v) in map {
                        if v.is_null() {
                            continue;
                        }

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

        let mut req = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("X-Pinecone-API-Version", &self.api_version);

        if self.use_auth {
            req = req.header("Api-Key", self.api_key.as_ref().unwrap());
        }

        let resp = req.json(&payload).send()?;
        if resp.status().is_success() {
            let j: Value = resp.json()?;
            let count = j
                .get("upsertedCount")
                .and_then(|c| c.as_u64())
                .unwrap_or(0);
            info!("Pinecone: upserted {} vectors into `{}`", count, self.namespace);
            Ok(())
        } else {
            let status = resp.status();
            let txt = resp.text()?;
            error!("Pinecone bulk upsert failed ({}): {}", status, txt);
            Err(format!("Pinecone bulk upsert error: {}", txt).into())
        }
    }
}
