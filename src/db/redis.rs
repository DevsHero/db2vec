use redis::Client;
use serde_json::Value;
use super::{ Database, DbError };
pub struct RedisDatabase {
    client: Client,
    password: Option<String>,
    args: crate::cli::Args,
}

impl RedisDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let client = Client::open(args.host.as_str()).map_err(|e| Box::new(e) as DbError)?;
        let password = if args.use_auth && !args.pass.is_empty() {
            Some(args.pass.clone())
        } else {
            None
        };
        let mut conn = client.get_connection().map_err(|e| Box::new(e) as DbError)?;

        if let Some(ref pass) = password {
            let _: () = redis
                ::cmd("AUTH")
                .arg(pass)
                .query(&mut conn)
                .map_err(|e| Box::new(e) as DbError)?;
        }

        Ok(RedisDatabase {
            client,
            password,
            args: args.clone(),
        })
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

        let mut con = self.client.get_connection()?;

        if let Some(ref pass) = self.password {
            let _: () = redis::cmd("AUTH").arg(pass).query(&mut con)?;
        }

        if self.args.group_redis {
            let table_key = format!("table:{}", table);
            let exists: bool = redis::cmd("EXISTS").arg(&table_key).query(&mut con)?;
            if exists {
                redis::cmd("DEL").arg(&table_key).query::<()>(&mut con)?;
            }

            redis::cmd("JSON.SET").arg(&table_key).arg("$").arg("{}").query::<()>(&mut con)?;
            for (id, vec, data) in items {
                let mut record_obj = serde_json::Map::new();
                record_obj.insert("id".to_string(), Value::String(id.clone()));
                record_obj.insert("vector".to_string(), serde_json::to_value(vec)?);

                if let Value::Object(obj) = data {
                    for (k, v) in obj {
                        record_obj.insert(k.clone(), v.clone());
                    }
                }

                redis
                    ::cmd("JSON.SET")
                    .arg(&table_key)
                    .arg(format!("$.{}", id.replace("-", "_")))
                    .arg(serde_json::to_string(&Value::Object(record_obj))?)
                    .query::<()>(&mut con)?;
            }
        } else {
            for (id, vec, data) in items {
                let key = id.clone();
                let exists: bool = redis::cmd("EXISTS").arg(&key).query(&mut con)?;
                if exists {
                    redis::cmd("DEL").arg(&key).query::<()>(&mut con)?;
                }

                let mut record_obj = serde_json::Map::new();
                record_obj.insert("vector".to_string(), serde_json::to_value(vec)?);
                record_obj.insert("label".to_string(), Value::String(format!("table:{}", table)));

                if let Value::Object(obj) = data {
                    for (k, v) in obj {
                        record_obj.insert(k.clone(), v.clone());
                    }
                }

                redis
                    ::cmd("JSON.SET")
                    .arg(&key)
                    .arg("$")
                    .arg(serde_json::to_string(&Value::Object(record_obj))?)
                    .query::<()>(&mut con)?;
            }
        }

        Ok(())
    }
}
