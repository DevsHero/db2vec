use byteorder::{ LittleEndian, WriteBytesExt };
use redis::{ Client, Commands };
use serde_json::Value;
use std::error::Error;
use super::Database;
pub struct RedisDatabase {
    client: Client,
    password: Option<String>,
}

impl RedisDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, Box<dyn Error>> {
        let client = Client::open(args.host.as_str())?;
        let password = if args.use_auth && !args.pass.is_empty() {
            Some(args.pass.clone())
        } else {
            None
        };
        Ok(RedisDatabase { client, password })
    }
}

impl Database for RedisDatabase {
    fn connect(_url: &str) -> Result<Self, Box<dyn Error>> where Self: Sized {
        unimplemented!("Use RedisDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        table: &str,
        key: &str,
        vector: &[f32],
        data: &Value
    ) -> Result<(), Box<dyn Error>> {
        let mut con = self.client.get_connection()?;
        if let Some(ref pass) = self.password {
            let _: () = redis::cmd("AUTH").arg(pass).query(&mut con)?;
        }
        let redis_key = format!("{}:{}", table, key);
        let vector_bytes = f32_vec_to_bytes(vector);
        let _: () = con.hset(&redis_key, "vector", vector_bytes)?;
        let _: () = con.hset(&redis_key, "data", serde_json::to_string(data)?)?;
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
