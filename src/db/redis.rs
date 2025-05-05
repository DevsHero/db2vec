use redis::Client;
use serde_json::Value;
use log::{ info, warn, debug };
use std::io::{ Error as IoError, ErrorKind };
use super::{ Database, DbError };

pub struct RedisDatabase {
    client: Client,
    password: Option<String>,
    dimension: usize,
    metric: String,
    group_redis: bool,
}

impl RedisDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        info!("Connecting to Redis at {}", args.vector_host);
        let client = Client::open(args.vector_host.as_str()).map_err(
            |e|
                Box::new(
                    IoError::new(ErrorKind::Other, format!("Failed to open Redis client: {}", e))
                ) as DbError
        )?;
        let password = if args.use_auth && !args.pass.is_empty() {
            Some(args.pass.clone())
        } else {
            None
        };

        let mut conn = client
            .get_connection()
            .map_err(
                |e|
                    Box::new(
                        IoError::new(
                            ErrorKind::Other,
                            format!("Failed to get Redis connection: {}", e)
                        )
                    ) as DbError
            )?;
        if let Some(ref pass) = password {
            redis
                ::cmd("AUTH")
                .arg(pass)
                .query::<()>(&mut conn)
                .map_err(
                    |e|
                        Box::new(
                            IoError::new(ErrorKind::Other, format!("Redis AUTH failed: {}", e))
                        ) as DbError
                )?;
            info!("Redis AUTH successful");
        }

        let pong: String = redis
            ::cmd("PING")
            .query(&mut conn)
            .map_err(
                |e|
                    Box::new(
                        IoError::new(ErrorKind::Other, format!("Redis PING failed: {}", e))
                    ) as DbError
            )?;
        if pong != "PONG" {
            warn!("Redis PING received unexpected response: {}", pong);
        } else {
            info!("Redis PING successful");
        }

        Ok(RedisDatabase {
            client,
            password,
            dimension: args.dimension,
            metric: args.metric.clone(),
            group_redis: args.group_redis,
        })
    }

    fn get_connection(&self) -> Result<redis::Connection, DbError> {
        let mut con = self.client
            .get_connection()
            .map_err(
                |e|
                    Box::new(
                        IoError::new(
                            ErrorKind::Other,
                            format!("Failed to get Redis connection: {}", e)
                        )
                    ) as DbError
            )?;
        if let Some(ref pass) = self.password {
            redis
                ::cmd("AUTH")
                .arg(pass)
                .query::<()>(&mut con)
                .map_err(
                    |e|
                        Box::new(
                            IoError::new(ErrorKind::Other, format!("Redis AUTH failed: {}", e))
                        ) as DbError
                )?;
        }
        Ok(con)
    }

    fn map_metric_to_redis(&self) -> &str {
        match self.metric.to_lowercase().as_str() {
            "cosine" => "COSINE",
            "l2" | "euclidean" => "L2",
            "ip" | "dotproduct" | "innerproduct" => "IP",
            _ => {
                warn!("Unsupported metric '{}', defaulting to COSINE", self.metric);
                "COSINE"
            }
        }
    }

    fn ensure_index_exists(
        &self,
        con: &mut redis::Connection,
        table: &str,
        sample_data: Option<&Value>
    ) -> Result<(), DbError> {
        let index_name = format!("idx:{}", table);

        match redis::cmd("FT.INFO").arg(&index_name).query::<Vec<redis::Value>>(con) {
            Ok(_) => {
                return Ok(());
            }
            Err(_) => {
                info!("Index '{}' not found, creating it now", index_name);
            }
        }

        let mut ft = redis::cmd("FT.CREATE");
        ft.arg(&index_name)
            .arg("ON")
            .arg("JSON")
            .arg("PREFIX")
            .arg("1")
            .arg(format!("item:{}:", table))
            .arg("SCHEMA")
            .arg("$.vector")
            .arg("AS")
            .arg("vector")
            .arg("VECTOR")
            .arg("FLAT")
            .arg("6")
            .arg("TYPE")
            .arg("FLOAT32")
            .arg("DIM")
            .arg(self.dimension.to_string())
            .arg("DISTANCE_METRIC")
            .arg(self.map_metric_to_redis());

        if let Some(Value::Object(data_map)) = sample_data {
            info!("Attempting to discover schema from first item data for index '{}'", index_name);
            let standard_fields = vec![
                ("source_table".to_string(), "TEXT".to_string()),
                ("original_id".to_string(), "TEXT".to_string())
            ];

            for (field, idx_ty_str) in standard_fields {
                debug!("Adding standard field to schema: $.{} AS {} {}", field, field, idx_ty_str);
                ft.arg(format!("$.{}", field)).arg("AS").arg(&field).arg(&idx_ty_str);
                if idx_ty_str == "TEXT" {
                    ft.arg("SORTABLE");
                }
            }

            for (field, value) in data_map {
                if field == "vector" || field == "source_table" || field == "original_id" {
                    continue;
                }

                let idx_ty = match value {
                    Value::String(_) => "TEXT",
                    Value::Number(_) => "NUMERIC",
                    Value::Bool(_) => "NUMERIC",
                    _ => {
                        continue;
                    }
                };

                debug!("Adding discovered field to schema: $.{} AS {} {}", field, field, idx_ty);
                ft.arg(format!("$.{}", field)).arg("AS").arg(field).arg(idx_ty);
                if idx_ty == "TEXT" {
                    ft.arg("SORTABLE");
                }
            }
        } else {
            warn!("No sample data provided or sample data is not a JSON object for index '{}'. Only indexing vector field.", index_name);
            ft.arg("$.source_table").arg("AS").arg("source_table").arg("TEXT").arg("SORTABLE");
            ft.arg("$.original_id").arg("AS").arg("original_id").arg("TEXT").arg("SORTABLE");
        }

        match ft.query::<()>(con) {
            Ok(_) => {
                info!("Created Redis index '{}'", index_name);
                Ok(())
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Index already exists") {
                    info!("Index '{}' already exists (concurrent creation?), skipping", index_name);
                    Ok(())
                } else {
                    Err(
                        Box::new(
                            IoError::new(
                                ErrorKind::Other,
                                format!("FT.CREATE failed for index '{}': {}", index_name, msg)
                            )
                        ) as DbError
                    )
                }
            }
        }
    }
}

impl Database for RedisDatabase {
    fn connect(_url: &str) -> Result<Self, DbError> where Self: Sized {
        unimplemented!("Use RedisDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        table: &str,
        items: &[(String, Vec<f32>, Value)]
    ) -> Result<(), DbError> {
        if items.is_empty() {
            return Ok(());
        }

        let normalized_table = table.to_lowercase();
        if normalized_table != table {
            info!("Normalizing Redis table/index name '{}' to '{}'", table, normalized_table);
        }

        let mut con = self.get_connection()?;

        if self.group_redis {
            let key = format!("table:{}", normalized_table);

            let docs: Vec<Value> = items
                .iter()
                .map(|(id, vec, data)| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("id".to_string(), Value::String(id.clone()));
                    obj.insert("vector".to_string(), serde_json::to_value(vec).unwrap());
                    if let Value::Object(map) = data {
                        for (k, v) in map {
                            if k != "vector" {
                                obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                    Value::Object(obj)
                })
                .collect();

            let payload = Value::Array(docs);

            redis
                ::cmd("JSON.SET")
                .arg(&key)
                .arg("$")
                .arg(serde_json::to_string(&payload)?)
                .query::<()>(&mut con)
                .map_err(|e| {
                    Box::new(
                        IoError::new(
                            ErrorKind::Other,
                            format!("Redis JSON.SET failed for '{}': {}", key, e)
                        )
                    ) as DbError
                })?;

            info!("Stored {} items grouped for table '{}' (original: '{}')", 
                  items.len(), normalized_table, table);
            return Ok(());
        }

        let first_item_data = items.first().map(|(_, _, data)| data);
        self.ensure_index_exists(&mut con, &normalized_table, first_item_data)?;

        let mut pipe = redis::pipe();
        pipe.atomic();

        for (id, vec, data) in items {
            let key = format!("item:{}:{}", normalized_table, id);
            let mut record_obj = serde_json::Map::new();
            record_obj.insert("vector".to_string(), serde_json::to_value(vec)?);
            record_obj.insert("source_table".to_string(), Value::String(table.to_string()));
            record_obj.insert("original_id".to_string(), Value::String(id.clone()));

            if let Value::Object(obj) = data {
                for (k, v) in obj {
                    if k != "vector" && k != "source_table" {
                        record_obj.insert(k.clone(), v.clone());
                    }
                }
            }

            debug!(
                "Redis JSON document for {}: {}",
                key,
                serde_json::to_string(&Value::Object(record_obj.clone()))?
            );

            pipe.cmd("JSON.SET")
                .arg(&key)
                .arg("$")
                .arg(serde_json::to_string(&Value::Object(record_obj))?);
        }

        pipe
            .query::<()>(&mut con)
            .map_err(|e| {
                Box::new(
                    IoError::new(
                        ErrorKind::Other,
                        format!("Redis pipeline failed for table '{}': {}", table, e)
                    )
                ) as DbError
            })?;

        info!("Stored {} items for table '{}' (original: '{}') in Redis", 
              items.len(), normalized_table, table);
        Ok(())
    }
}
