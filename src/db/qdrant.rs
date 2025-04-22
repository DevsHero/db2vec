use log::{ info, warn };
use reqwest::blocking::Client;
use serde_json::{ json, Value };
use super::{ Database, DbError };

pub struct QdrantDatabase {
    client: Client,
    url: String,
    api_key: Option<String>,
    dimension: usize,
    metric: String,
}

impl QdrantDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let qdrant_url = args.host.clone();
        let api_key = if args.use_auth && !args.secret.is_empty() {
            Some(args.secret.clone())
        } else {
            None
        };
        let client = Client::new();

        Ok(QdrantDatabase {
            client,
            url: qdrant_url,
            api_key,
            dimension: args.dimension,
            metric: args.metric.clone(),
        })
    }
}

impl Database for QdrantDatabase {
    fn connect(url: &str) -> Result<Self, DbError> where Self: Sized {
        let client = Client::new();
        Ok(QdrantDatabase {
            client,
            url: url.to_string(),
            api_key: None,
            dimension: 768,
            metric: "cosine".to_string(),
        })
    }

    fn store_vector(
        &self,
        table: &str,
        items: &[(String, Vec<f32>, Value)]
    ) -> Result<(), DbError> {
        if items.is_empty() {
            return Ok(());
        }

        let coll_url = format!("{}/collections/{}", self.url, table);
        let mut chk = self.client.get(&coll_url);
        if let Some(k) = &self.api_key {
            chk = chk.header("api-key", k);
        }
        let resp = chk.send()?;
        if resp.status().as_u16() == 404 {
            let distance = match self.metric.to_lowercase().as_str() {
                "cosine" => "Cosine",
                "euclidean" => "Euclidean",
                "dotproduct" | "dot" => "Dot",
                other => {
                    warn!("Unknown metric '{}', falling back to Cosine", other);
                    "Cosine"
                }
            };

            info!(
                "Creating Qdrant collection '{}' with dimension {} and distance {}",
                table,
                self.dimension,
                distance
            );
            let body =
                json!({
                "vectors": {
                    "size": self.dimension,
                    "distance": distance
                }
            });
            let mut crt = self.client.put(&coll_url).json(&body);
            if let Some(k) = &self.api_key {
                crt = crt.header("api-key", k);
            }
            let cr = crt.send()?;
            if !cr.status().is_success() {
                let err = cr.text()?;
                return Err(format!("Failed to create collection '{}': {}", table, err).into());
            }
        }

        let points: Vec<Value> = items
            .iter()
            .map(|(id, vec, payload)| {
                let v = if vec.len() == self.dimension {
                    vec.clone()
                } else {
                    warn!(
                        "ID={}: vector length {} ≠ {}, filling zeros",
                        id,
                        vec.len(),
                        self.dimension
                    );
                    vec![0.0; self.dimension]
                };
                json!({ "id": id, "vector": v, "payload": payload })
            })
            .collect();

        let up_url = format!("{}/collections/{}/points?wait=true", self.url, table);
        let mut up = self.client.put(&up_url).json(&json!({ "points": points }));
        if let Some(k) = &self.api_key {
            up = up.header("api-key", k);
        }
        let up_res = up.send()?;
        if up_res.status().is_success() {
            info!("Qdrant: upserted {} points into `{}`", items.len(), table);
            Ok(())
        } else {
            let txt = up_res.text()?;
            Err(format!("Qdrant upsert failed: {}", txt).into())
        }
    }
}
