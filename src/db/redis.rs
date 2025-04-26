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
        info!("Connecting to Redis at {}", args.host);
        let client = Client::open(args.host.as_str()).map_err(
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

    fn ensure_index_exists(&self, con: &mut redis::Connection, table: &str) -> Result<(), DbError> {
        let index_name = format!("idx:{}", table);
        debug!("Checking if Redis index '{}' exists", index_name);

        let info_result: redis::RedisResult<Vec<redis::Value>> = redis
            ::cmd("FT.INFO")
            .arg(&index_name)
            .query(con);

        match info_result {
            Ok(_) => {
                info!("Redis index '{}' already exists.", index_name);
                Ok(())
            }
            Err(_e) => {
                info!("Creating Redis index '{}' with dimension {}", index_name, self.dimension);
                let mut ft_create_cmd = redis::cmd("FT.CREATE");
                ft_create_cmd
                    .arg(&index_name)
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
                    .arg(self.map_metric_to_redis())
                    .arg("$.table_meta")
                    .arg("AS")
                    .arg("table_tag")
                    .arg("TAG");

                match ft_create_cmd.query::<()>(con) {
                    Ok(_) => {
                        info!("Successfully created Redis index '{}'", index_name);
                        Ok(())
                    }
                    Err(e) => {
                        if e.to_string().contains("Index already exists") {
                            info!("Index '{}' already exists (concurrent creation?)", index_name);
                            Ok(())
                        } else {
                            Err(
                                Box::new(
                                    IoError::new(
                                        ErrorKind::Other,
                                        format!(
                                            "Failed to create Redis index '{}': {}",
                                            index_name,
                                            e
                                        )
                                    )
                                ) as DbError
                            )
                        }
                    }
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

        let mut con = self.get_connection()?;

        if self.group_redis {
            let key = format!("table:{}", table);

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

            info!("Stored {} items grouped for table '{}'", items.len(), table);
            return Ok(());
        }

        self.ensure_index_exists(&mut con, table)?;

        let mut pipe = redis::pipe();
        pipe.atomic();

        for (id, vec, data) in items {
            let key = format!("item:{}:{}", table, id);

            let mut record_obj = serde_json::Map::new();
            record_obj.insert("vector".to_string(), serde_json::to_value(vec)?);
            record_obj.insert("table_meta".to_string(), Value::String(table.to_string()));
            record_obj.insert("original_id".to_string(), Value::String(id.clone()));

            if let Value::Object(obj) = data {
                for (k, v) in obj {
                    if k != "vector" {
                        record_obj.insert(k.clone(), v.clone());
                    }
                }
            }

            pipe.cmd("JSON.SET")
                .arg(&key)
                .arg("$")
                .arg(serde_json::to_string(&Value::Object(record_obj))?);
        }

        pipe
            .query::<()>(&mut con)
            .map_err(
                |e|
                    Box::new(
                        IoError::new(
                            ErrorKind::Other,
                            format!("Redis pipeline failed for table '{}': {}", table, e)
                        )
                    ) as DbError
            )?;

        info!("Stored {} items for table '{}' in Redis", items.len(), table);

        Ok(())
    }
}
