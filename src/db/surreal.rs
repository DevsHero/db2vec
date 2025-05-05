use base64::{ engine::general_purpose::STANDARD, Engine as _ };
use log::{ info, error, warn };
use reqwest::blocking::Client;
use serde_json::Value;
use super::{ Database, DbError };

pub struct SurrealDatabase {
    url: String,
    ns: String,
    db: String,
    auth_header: Option<String>,
    client: Client,
}

impl SurrealDatabase {
    pub fn new(args: &crate::cli::Args) -> Result<Self, DbError> {
        let base_url = args.vector_host.clone();
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
        info!("Sending DEFINE NAMESPACE: {}", define_ns_sql);
        let mut req_ns = client
            .post(&sql_url)
            .header("Content-Type", "text/plain")
            .header("Accept", "application/json")
            .body(define_ns_sql);

        if let Some(ref auth) = auth_header {
            req_ns = req_ns.header("Authorization", auth);
        }

        let resp_ns = req_ns.send().map_err(|e| Box::new(e) as DbError)?;
        let status_ns = resp_ns.status();
        let text_ns = resp_ns.text().map_err(|e| Box::new(e) as DbError)?;

        info!("SurrealDB DEFINE NAMESPACE response: {}", text_ns);
        if !status_ns.is_success() && !text_ns.contains("already exists") {
            error!("Failed to execute DEFINE NAMESPACE (Status: {}): {}", status_ns, text_ns);
        }

        let define_db_sql = format!("DEFINE DATABASE IF NOT EXISTS {};", db);
        info!("Sending DEFINE DATABASE: {}", define_db_sql);
        let mut req_db = client
            .post(&sql_url)
            .header("Content-Type", "text/plain")
            .header("Accept", "application/json")
            .header("Surreal-NS", &ns)
            .body(define_db_sql);

        if let Some(ref auth) = auth_header {
            req_db = req_db.header("Authorization", auth);
        }

        let resp_db = req_db.send().map_err(|e| Box::new(e) as DbError)?;
        let status_db = resp_db.status();
        let text_db = resp_db.text().map_err(|e| Box::new(e) as DbError)?;

        info!("SurrealDB DEFINE DATABASE response: {}", text_db);
        if !status_db.is_success() && !text_db.contains("already exists") {
            error!("Failed to execute DEFINE DATABASE (Status: {}): {}", status_db, text_db);
        }

        Ok(SurrealDatabase { url: base_url, ns, db, auth_header, client })
    }

    fn ensure_table_exists(&self, table: &str) -> Result<(), DbError> {
        let sql_url = format!("{}/sql", self.url.trim_end_matches('/'));
        let define_table_sql =
            format!("DEFINE TABLE IF NOT EXISTS `{}` TYPE ANY SCHEMALESS PERMISSIONS NONE;", table);

        info!("Ensuring table exists: {}", define_table_sql);

        let mut req = self.client
            .post(&sql_url)
            .header("Content-Type", "text/plain")
            .header("Accept", "application/json")
            .header("Surreal-NS", &self.ns)
            .header("Surreal-DB", &self.db)
            .body(define_table_sql);

        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().map_err(|e| Box::new(e) as DbError)?;
        let status = resp.status();
        let text = resp.text().map_err(|e| Box::new(e) as DbError)?;

        info!("SurrealDB DEFINE TABLE response ({}): {}", status, text);

        if !status.is_success() {
            warn!("Potential issue defining table '{}' (Status: {}): {}", table, status, text);
        }

        Ok(())
    }

  }

impl Database for SurrealDatabase {
    fn connect(_url: &str) -> Result<Self, DbError> where Self: Sized {
        unimplemented!("Use SurrealDatabase::new(&args) instead");
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
            info!("Normalizing SurrealDB table name '{}' to '{}'", table, normalized_table);
        }

        self.ensure_table_exists(&normalized_table)?;

        let records: Vec<(String, Value)> = items
            .iter()
            .map(|(id, vector, meta)| {
                let mut record = meta.clone();
                if let Some(obj) = record.as_object_mut() {
                    obj.insert(
                        "vector".to_string(),
                        serde_json::to_value(vector).unwrap_or_default()
                    );
                    obj.insert("original_table".to_string(), Value::String(table.to_string()));
                }
                (id.clone(), record)
            })
            .collect();

        let import_url = format!("{}/import", self.url.trim_end_matches('/'));
        let mut import_data = String::new();

        for (id, data) in &records {
            let record_id = format!("{}:`{}`", normalized_table, id);
            let content_json = serde_json::to_string(&data)?;
            import_data.push_str(&format!("CREATE {} CONTENT {};\n", record_id, content_json));
        }

        info!("SurrealDB Import URL: {}", import_url);
        let preview_len = import_data.chars().count().min(300);
        info!(
            "SurrealDB Import Payload Preview: {}...",
            import_data.chars().take(preview_len).collect::<String>()
        );

        let mut req = self.client
            .post(&import_url)
            .header("Surreal-NS", &self.ns)
            .header("Surreal-DB", &self.db)
            .header("Content-Type", "text/plain")
            .header("Accept", "application/json")
            .body(import_data);

        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }

        let resp = req.send()?;
        let status = resp.status();
        let text = resp.text().unwrap_or_else(|e| format!("Failed to read response body: {}", e));
        info!("SurrealDB Import Response Status: {}", status);
        info!("SurrealDB Import Response Body: {}", text);

        if !status.is_success() {
            return Err(format!("SurrealDB batch import failed ({}): {}", status, text).into());
        }

        info!("SurrealDB: successfully imported {} records to {} (original: {})", 
              records.len(), normalized_table, table);
        Ok(())
    }
}
