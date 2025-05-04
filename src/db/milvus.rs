use reqwest::blocking::Client;
use serde_json::{ json, Value };
use super::{ Database, DbError };
use log::{ debug, error, info, warn };

pub struct MilvusDatabase {
    url: String,
    token: Option<String>,
    client: Client,
    dimension: usize,
    db_name: String,
    metric: String,
}

impl MilvusDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let url = args.vector_host.trim_end_matches('/').to_string();
        let db_name = args.database.clone();
        let token = if args.use_auth {
            if !args.secret.is_empty() {
                Some(args.secret.clone())
            } else if !args.user.is_empty() || !args.pass.is_empty() {
                Some(format!("{}:{}", args.user, args.pass))
            } else {
                None
            }
        } else {
            None
        };
        let client = Client::new();

        let metric = match args.metric.to_uppercase().as_str() {
            "COSINE" | "COSINE_SIMILARITY" => "COSINE".to_string(),
            "IP" | "DOT_PRODUCT" => "IP".to_string(),
            "L2" | "EUCLIDEAN" => "L2".to_string(),
            _ => {
                return Err(
                    format!(
                        "Invalid metric type '{}'. Use COSINE, IP (DOT_PRODUCT), or L2 (EUCLIDEAN).",
                        args.metric
                    ).into()
                );
            }
        };

        Ok(MilvusDatabase {
            url,
            token,
            client,
            dimension: args.dimension,
            db_name,
            metric,
        })
    }

    fn add_auth(
        &self,
        req_builder: reqwest::blocking::RequestBuilder
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(ref t) = self.token { req_builder.bearer_auth(t) } else { req_builder }
    }

    fn send_request(
        &self,
        req_builder: reqwest::blocking::RequestBuilder,
        context: &str
    ) -> Result<Value, DbError> {
        let request = req_builder
            .build()
            .map_err(|e| format!("Failed to build request for {}: {}", context, e))?;
        debug!("Sending Milvus request: {} {}", request.method(), request.url());

        let response = self.client
            .execute(request)
            .map_err(|e| format!("Failed to send request for {}: {}", context, e))?;
        let status = response.status();
        let text = response
            .text()
            .map_err(|e| format!("Failed to read response body for {}: {}", context, e))?;
        debug!("Milvus raw response for {} ({}): {}", context, status, text);

        let json_value: Value = serde_json
            ::from_str(&text)
            .map_err(|e|
                format!(
                    "Failed to parse JSON response for {} ({}): {}\nBody: {}",
                    context,
                    status,
                    e,
                    text
                )
            )?;

        if let Some(code) = json_value.get("code").and_then(|c| c.as_i64()) {
            if code != 0 {
                let message = json_value
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown Milvus API error");
                error!("Milvus API error for {}: Code {} - {}", context, code, message);
                return Err(
                    format!("Milvus API error for {}: Code {} - {}", context, code, message).into()
                );
            }
        } else if !status.is_success() {
            return Err(
                format!("Milvus request failed for {} ({}): {}", context, status, text).into()
            );
        }

        Ok(json_value)
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

        let list_db_url = format!("{}/v2/vectordb/databases/list", self.url);
        let list_db_req = self.client.post(&list_db_url).json(&json!({}));
        let list_db_resp_val = self.send_request(self.add_auth(list_db_req), "list databases")?;

        let db_exists = list_db_resp_val
            .get("data")
            .and_then(|data| data.as_array())
            .map(|dbs| {
                dbs.iter().any(|db_name_val| { db_name_val.as_str() == Some(&self.db_name) })
            })
            .unwrap_or(false);

        if !db_exists {
            info!("Database '{}' not found. Creating...", self.db_name);
            let create_db_url = format!("{}/v2/vectordb/databases/create", self.url);
            let payload = json!({"dbName": self.db_name});
            let create_db_req = self.client.post(&create_db_url).json(&payload);
            self.send_request(self.add_auth(create_db_req), "create database")?;
            info!("Milvus database '{}' created successfully.", self.db_name);
        } else {
            info!("Milvus database '{}' already exists.", self.db_name);
        }

        let stats_url = format!("{}/v2/vectordb/collections/get_stats", self.url);
        let stats_payload =
            json!({
            "dbName": self.db_name,
            "collectionName": table
        });
        let stats_req = self.client.post(&stats_url).json(&stats_payload);

        let stats_resp = self
            .add_auth(stats_req)
            .send()
            .map_err(|e| format!("Failed to send get_stats request: {}", e))?;
        let stats_status = stats_resp.status();
        let stats_text = stats_resp
            .text()
            .map_err(|e| format!("Failed to read get_stats response body: {}", e))?;
        debug!("Milvus raw response for get_stats ({}): {}", stats_status, stats_text);

        let stats_json: Value = serde_json
            ::from_str(&stats_text)
            .map_err(|e|
                format!(
                    "Failed to parse get_stats JSON ({}): {}\nBody: {}",
                    stats_status,
                    e,
                    stats_text
                )
            )?;

        let collection_exists = stats_json.get("code").and_then(|c| c.as_i64()) == Some(0);

        if !collection_exists {
            let is_not_found_error = stats_json
                .get("message")
                .and_then(|m| m.as_str())
                .map(
                    |msg|
                        msg.contains("collection not found") ||
                        msg.contains("collection does not exist")
                )
                .unwrap_or(false);

            if stats_json.get("code").and_then(|c| c.as_i64()) != Some(0) && !is_not_found_error {
                let code = stats_json
                    .get("code")
                    .and_then(|c| c.as_i64())
                    .unwrap_or(-1);
                let message = stats_json
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error checking collection stats");
                error!(
                    "Failed to check stats for collection '{}' in db '{}': Code {} - {}",
                    table,
                    self.db_name,
                    code,
                    message
                );
                return Err(
                    format!(
                        "Failed to check stats for collection '{}' in db '{}': Code {} - {}",
                        table,
                        self.db_name,
                        code,
                        message
                    ).into()
                );
            }

            info!("Collection '{}' not found in database '{}'. Creating...", table, self.db_name);
            let create_coll_url = format!("{}/v2/vectordb/collections/create", self.url);

            let create_coll_payload =
                json!({
                "dbName": self.db_name,
                "collectionName": table,
                "schema": {
                    "autoId": false,
                    "enableDynamicField": true,
                    "fields": [
                        {
                            "fieldName": "id",
                            "dataType": "VarChar",
                            "isPrimary": true,
                            "elementTypeParams": {
                                "max_length": 256 
                            }
                        },
                        {
                            "fieldName": "vector",
                            "dataType": "FloatVector",
                            "elementTypeParams": {
                                "dim": self.dimension.to_string()
                            }
                        }
                    ]
                },
                "indexParams": [
                    {
                        "fieldName": "vector",
                        "metricType": self.metric
                    }
                ]
            });

            let create_coll_req = self.client.post(&create_coll_url).json(&create_coll_payload);
            self.send_request(self.add_auth(create_coll_req), "create collection")?;
            info!(
                "Milvus collection '{}' created successfully in database '{}'.",
                table,
                self.db_name
            );
        } else {
            info!("Milvus collection '{}' already exists in database '{}'.", table, self.db_name);
        }

        let data: Vec<Value> = items
            .iter()
            .map(|(id, vec, meta)| {
                let v = if vec.len() == self.dimension {
                    vec.clone()
                } else {
                    warn!(
                        "ID='{}' in collection '{}': vector length {} ≠ {}, filling with zeros",
                        id,
                        table,
                        vec.len(),
                        self.dimension
                    );
                    vec![0.0; self.dimension]
                };

                let mut entity_obj =
                    json!({
                    "id": id,
                    "vector": v
                });

                if let Some(map) = meta.as_object() {
                    for (k, v_meta) in map.iter() {
                        if k != "id" && k != "vector" {
                            let clean_key = k.replace('.', "_");
                            if clean_key != k.as_str() {
                                warn!(
                                    "Metadata key '{}' renamed to '{}' for Milvus compatibility.",
                                    k,
                                    clean_key
                                );
                            }
                            entity_obj[clean_key] = v_meta.clone();
                        }
                    }
                }
                entity_obj
            })
            .collect();

        let insert_url = format!("{}/v2/vectordb/entities/insert", self.url);
        let insert_payload =
            json!({
            "dbName": self.db_name,
            "collectionName": table,
            "data": data
        });

        let insert_req = self.client.post(&insert_url).json(&insert_payload);
        self.send_request(self.add_auth(insert_req), "insert entities")?;
        info!(
            "Milvus: inserted {} entities into collection '{}' in database '{}'.",
            items.len(),
            table,
            self.db_name
        );

        Ok(())
    }
}
