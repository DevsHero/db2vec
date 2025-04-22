use byteorder::{ LittleEndian, WriteBytesExt };
use redis::Client;
use serde_json::Value;
use super::{ Database, DbError };
pub struct RedisDatabase {
    client: Client,
    password: Option<String>,
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

        Ok(RedisDatabase { client, password })
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
        let mut con = self.client.get_connection()?;

        if let Some(ref pass) = self.password {
            let _: () = redis::cmd("AUTH").arg(pass).query(&mut con)?;
        }

        let mut pipe = redis::pipe();

        for (id, vec, data) in items {
            let key = format!("{}:{}", table, id);
            let bytes = f32_vec_to_bytes(vec);
            pipe.hset(&key, "vector", bytes)
                .ignore()
                .hset(&key, "data", serde_json::to_string(data)?)
                .ignore();
        }
        let _ = pipe.exec(&mut con);
        Ok(())
    }
}

fn f32_vec_to_bytes(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for &v in vec {
        bytes.write_f32::<LittleEndian>(v).unwrap();
    }
    bytes
}
