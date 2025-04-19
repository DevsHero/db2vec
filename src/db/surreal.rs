use base64::{ engine::general_purpose::STANDARD, Engine as _ };
use reqwest::blocking::Client;
use serde_json::Value;
use std::error::Error;

use super::Database;

pub struct SurrealDatabase {
    url: String,
    ns: String,
    db: String,
    auth_header: Option<String>,
    client: Client,
}

impl SurrealDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, Box<dyn Error>> {
        let base_url = args.host.clone();
        let sql_url = format!("{}/sql", base_url.trim_end_matches('/'));
        let ns = args.namespace.clone();
        let db = args.database.clone();
        let client = Client::new();

        let auth_header = if args.use_auth {
            Some(format!("Basic {}", STANDARD.encode(format!("{}:{}", args.user, args.pass))))
        } else {
            None
        };

        let define_ns_sql = format!("DEFINE NAMESPACE IF NOT EXISTS {};", ns);
        println!("Sending DEFINE NAMESPACE: {}", define_ns_sql);
        let mut req_ns = client
            .post(&sql_url)
            .header("Content-Type", "text/plain")
            .header("Accept", "application/json")
            .body(define_ns_sql);
        if let Some(ref auth) = auth_header {
            req_ns = req_ns.header("Authorization", auth);
        }
        let resp_ns = req_ns.send()?;
        let status_ns = resp_ns.status();
        let text_ns = resp_ns.text()?;
        println!("SurrealDB DEFINE NAMESPACE response: {}", text_ns);
        if !status_ns.is_success() && !text_ns.contains("already exists") {
            eprintln!("Failed to execute DEFINE NAMESPACE (Status: {}): {}", status_ns, text_ns);
        }

        let define_db_sql = format!("DEFINE DATABASE IF NOT EXISTS {};", db);
        println!("Sending DEFINE DATABASE: {}", define_db_sql);
        let mut req_db = client
            .post(&sql_url)
            .header("Content-Type", "text/plain")
            .header("Accept", "application/json")
            .header("Surreal-NS", &ns)
            .body(define_db_sql);
        if let Some(ref auth) = auth_header {
            req_db = req_db.header("Authorization", auth);
        }
        let resp_db = req_db.send()?;
        let status_db = resp_db.status();
        let text_db = resp_db.text()?;
        println!("SurrealDB DEFINE DATABASE response: {}", text_db);
        if !status_db.is_success() && !text_db.contains("already exists") {
            eprintln!("Failed to execute DEFINE DATABASE (Status: {}): {}", status_db, text_db);
        }

        Ok(SurrealDatabase { url: base_url, ns, db, auth_header, client })
    }

    pub fn create_record(
        &self,
        table: &str,
        key: &str,
        data: &Value
    ) -> Result<(), Box<dyn Error>> {
        let mut record = data.clone();
        if let Some(obj) = record.as_object_mut() {
            obj.insert("id".to_string(), format!("{}:{}", table, key).into());
        }
        let json_body = serde_json::to_string(&record)?;
        let key_url = format!("{}/key/{}", self.url.trim_end_matches("/sql"), table);
        let mut req = self.client
            .post(&key_url)
            .header("Surreal-NS", &self.ns)
            .header("Surreal-DB", &self.db)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(json_body);

        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req.send()?;
        let status = resp.status();
        let text = resp.text()?;
        println!("SurrealDB create response: {}", status);
        if !status.is_success() {
            eprintln!("Failed to create record (Status: {}): {}", status, text);
        }
        Ok(())
    }
}

impl Database for SurrealDatabase {
    fn connect(_url: &str) -> Result<Self, Box<dyn Error>> where Self: Sized {
        unimplemented!("Use SurrealDatabase::new(&args) instead");
    }

    fn store_vector(
        &self,
        table: &str,
        key: &str,
        vector: &[f32],
        data: &Value
    ) -> Result<(), Box<dyn Error>> {
        let mut record = data.clone();
        if let Some(obj) = record.as_object_mut() {
            obj.insert("vector".to_string(), serde_json::to_value(vector)?);
        }
        self.create_record(table, key, &record)
    }
}
