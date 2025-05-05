use std::path::Path;
use std::process::Command;
use std::time::Duration;
use std::thread; 
use std::error::Error as StdError; 
use db2vec::cli::Args;
use db2vec::embedding::models::google::GoogleEmbeddingClient;
use db2vec::embedding::models::ollama::OllamaEmbeddingClient;
use db2vec::embedding::models::tei::TeiEmbeddingClient;
use db2vec::parser::parse_database_export;
use db2vec::db::Database;
use db2vec::embedding::AsyncEmbeddingGenerator;
use db2vec::util::utils::read_file_content;
use uuid::Uuid; 
use std::sync::OnceLock;
use db2vec::util::handle_tei::{start_and_wait_for_tei, ManagedProcess};
use async_trait::async_trait;
use tokio::runtime::Runtime;
use portpicker;
static TEI_PROCESS: OnceLock<Option<ManagedProcess>> = OnceLock::new();

#[derive(Debug, Clone)]
enum TestEmbeddingProvider {
    Mock,
    Ollama,
    Tei,
    Google,
}

fn create_embedding_provider(provider_type: TestEmbeddingProvider, args: &Args) 
    -> Result<Box<dyn AsyncEmbeddingGenerator>, String> {
    
    match provider_type {
        TestEmbeddingProvider::Mock => {
            Ok(Box::new(MockEmbeddingGenerator::new(args.dimension)))
        },
        TestEmbeddingProvider::Ollama => {
            if is_ollama_available() {
                let ollama = OllamaEmbeddingClient::new(
                    "http://localhost:11434",
                    &args.embedding_model,     
                    args.dimension          
                ).map_err(|e| format!("Failed to create Ollama provider: {}", e))?;
                
                Ok(Box::new(ollama))
            } else {
                println!("Ollama not available, falling back to mock embeddings");
                Ok(Box::new(MockEmbeddingGenerator::new(args.dimension)))
            }
        },
        TestEmbeddingProvider::Tei => {
            let tei_path = Path::new("tei/tei-metal");
            if tei_path.exists() {
                if std::env::var("FORCE_TEI").is_ok() {
                    let random_port = portpicker::pick_unused_port().expect("No ports free");
                    println!("🔌 Using dynamic port for TEI: {}", random_port);
                    
                    match start_and_wait_for_tei(&Args {
                        tei_binary_path: tei_path.to_string_lossy().to_string(),
                        embedding_model: args.embedding_model.clone(),
                        tei_local_port: random_port,
                        ..args.clone()
                    }) {
                        Ok((process, server_url)) => {
                            let tei = TeiEmbeddingClient::new(
                                server_url,
                                args.dimension,
                                60
                            ).map_err(|e| format!("Failed to create TEI provider: {}", e))?;
                            
                            let _ = TEI_PROCESS.set(Some(process));
                            return Ok(Box::new(tei));
                        },
                        Err(e) => {
                            println!("⚠️ TEI startup failed: {}. Falling back to mock embeddings.", e);
                        }
                    }
                } else {
                    println!("⚠️ TEI binary found but using mock embeddings for test stability");
                    println!("   Set FORCE_TEI=1 to use actual TEI embeddings");
                }
            } else {
                println!("⚠️ TEI binary not found, using mock embeddings");
            }
            Ok(Box::new(MockEmbeddingGenerator::new(args.dimension)))
        },
        TestEmbeddingProvider::Google => {
            if let Ok(api_key) = std::env::var("EMBEDDING_API_KEY") {
                let google = GoogleEmbeddingClient::new(
                    api_key,
                   Some( args.embedding_model.clone()),
                    args.dimension
                ).map_err(|e| format!("Failed to create Google provider: {}", e))?;
                
                Ok(Box::new(google))
            } else {
                println!("Google API key not found, falling back to mock embeddings");
                Ok(Box::new(MockEmbeddingGenerator::new(args.dimension)))
            }
        }
    }
}

fn is_ollama_available() -> bool {
    let client = reqwest::blocking::Client::new();
    println!("⚙️ Checking if Ollama is available at http://localhost:11434/api/embeddings...");
    
    let payload = r#"{"model":"nomic-embed-text","prompt":"test"}"#;
    
    match client.post("http://localhost:11434/api/embeddings")
        .header("Content-Type", "application/json")
        .body(payload)
        .send() 
    {
        Ok(response) => {
            println!("⚙️ Ollama response status: {}", response.status());
            let result = response.status().is_success();
            println!("⚙️ Ollama is available: {}", result);
            result
        },
        Err(e) => {
            println!("⚙️ Failed to connect to Ollama: {}", e);
            false
        },
    }
}

struct MockEmbeddingGenerator {
    dimension: usize,
}

impl MockEmbeddingGenerator {
    fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

#[async_trait]
impl AsyncEmbeddingGenerator for MockEmbeddingGenerator {
    async fn generate_embeddings_batch(
        &self,
        texts: &[String]
    ) -> Result<Vec<Vec<f32>>, Box<dyn StdError + Send + Sync>> {
        let mock_vectors = vec![vec![0.1; self.dimension]; texts.len()];
        Ok(mock_vectors)
    }

    fn get_dimension(&self) -> usize {
        self.dimension
    }
}

const SAMPLE_DIR: &str = "samples";
const TESTS_DIR: &str = "tests";
const TEST_DB_NAME: &str = "db2vec_test";

struct TestConfig {
    db_type: &'static str,
    host: &'static str,
    port: u16,
    container_name: &'static str,
    docker_cmd: &'static str,
}

fn cleanup_test_containers(db_configs: &[TestConfig]) {
    println!("Cleaning up test containers...");
    for config in db_configs {
        if config.db_type == "milvus" {
            let compose_file = Path::new(TESTS_DIR).join("docker-compose.yml");
            if compose_file.exists() {
                let _ = Command::new("docker-compose")
                    .args(["-f", compose_file.to_str().unwrap(), "down", "-v"])
                    .status();
                println!("  Removed Milvus containers");
            }
        } else {
         
            let _ = Command::new("docker")
                .args(["stop", config.container_name])
                .status();
            
            let _ = Command::new("docker")
                .args(["rm", "-f", config.container_name])
                .status();
            
            println!("  Removed container: {}", config.container_name);
        }
    }
    
    thread::sleep(Duration::from_secs(2));
    println!("Cleanup complete");
}

#[test]
fn test_all_database_combinations() {

    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Warn) 
        .is_test(true)
        .try_init();
    
    let db_configs = vec![
        TestConfig {
            db_type: "pinecone", 
            host: "http://localhost", 
            port: 12001,
            container_name: "dense-index-test",
            docker_cmd: "docker run -d \
                --name dense-index-test \
                -e PORT=5081 \
                -e INDEX_TYPE=serverless \
                -e VECTOR_TYPE=dense \
                -e DIMENSION=768 \
                -e METRIC=cosine \
                -p 12001:5081 \
                --platform linux/amd64 \
                ghcr.io/pinecone-io/pinecone-index:latest"
        },
        TestConfig {
            db_type: "surrealdb",
            host: "http://localhost",
            port: 12002,
            container_name: "surreal-test",
            docker_cmd: "docker run -d --rm \
                --name surreal-test \
                -p 12002:8000 \
                surrealdb/surrealdb:latest \
                start --user root --pass root"
        },
        TestConfig {
            db_type: "milvus",
            host: "http://localhost",
            port: 12003,  
            container_name: "milvus-standalone-test", 
            docker_cmd: "", 
        },
        TestConfig {
            db_type: "redis",
            host: "redis://localhost",
            port: 12004,
            container_name: "redis-stack-test",
            docker_cmd: "docker run -d \
                --name redis-stack-test \
                -p 12004:6379 \
                -p 11004:8001 \
                redis/redis-stack:latest"
        },
        TestConfig {
            db_type: "chroma",
            host: "http://localhost",
            port: 12005,
            container_name: "chromadb-test",
            docker_cmd: "docker run -d \
                --name chromadb-test \
                -v ./chroma-data:/data \
                -p 12005:8000 \
                chromadb/chroma"
        },
        TestConfig {
            db_type: "qdrant",
            host: "http://localhost",
            port: 12333,  
            container_name: "qdrant-test",
            docker_cmd: "docker run -d \
                --name qdrant-test \
                -p 12333:6333 \
                -p 12334:6334 \
                -v ./qdrant_storage:/qdrant/storage \
                qdrant/qdrant"
        },
    ];

    cleanup_test_containers(&db_configs);

    let sample_files = vec![
        ("mssql", "mssql_sample.sql"),
        ("mysql", "mysql_sample.sql"),
        ("oracle", "oracle_sample.sql"),
        ("postgres", "postgres_sample.sql"),
        ("sqlite", "sqlite_sample.sql"),
        ("surreal", "surreal_sample.surql"),
    ];

    for (_, filename) in &sample_files {
        let sample_path = Path::new(SAMPLE_DIR).join(filename);
        assert!(sample_path.exists(), "Sample file {} not found!", sample_path.display());
    }

    let specified_provider = std::env::var("EMBEDDING_PROVIDER").ok().map(|p| p.to_lowercase());
    
    let provider_type = match specified_provider.as_deref() {
        Some("google") => {
            if let Ok(api_key) = std::env::var("EMBEDDING_API_KEY") {
                println!("⚠️ Using Google API for embeddings (may incur costs)");
                TestEmbeddingProvider::Google
            } else {
                println!("⚠️ Google embedding provider specified but EMBEDDING_API_KEY not set");
                println!("Falling back to mock embeddings");
                TestEmbeddingProvider::Mock
            }
        },
        Some("ollama") => {
            if is_ollama_available() {
                println!("🚀 Using Ollama for embeddings");
                TestEmbeddingProvider::Ollama
            } else {
                println!("⚠️ Ollama embedding provider specified but Ollama not available");
                println!("Falling back to mock embeddings");
                TestEmbeddingProvider::Mock
            }
        },
        Some("tei") => {
            let tei_path = Path::new("tei/tei-metal");
            if tei_path.exists() {
                println!("🚀 Using TEI for embeddings");
                TestEmbeddingProvider::Tei
            } else {
                println!("⚠️ TEI embedding provider specified but TEI binary not found");
                println!("Falling back to mock embeddings");
                TestEmbeddingProvider::Mock
            }
        },
        Some("mock") => {
            println!("Using mock embeddings (no external API calls)");
            TestEmbeddingProvider::Mock
        },
        Some(unknown) => {
            println!("⚠️ Unknown embedding provider: '{}', using Mock", unknown);
            TestEmbeddingProvider::Mock
        },
        None => {
            if std::env::var("USE_GOOGLE_API").is_ok() && std::env::var("EMBEDDING_API_KEY").is_ok() {
                println!("⚠️ Using Google API for embeddings (may incur costs)");
                TestEmbeddingProvider::Google
            } else if is_ollama_available() {
                TestEmbeddingProvider::Ollama
            } else {
                println!("Using mock embeddings (no external API calls)");
                TestEmbeddingProvider::Mock
            }
        }
    };

    println!("🔍 EMBEDDING PROVIDER: {:?} 🔍", provider_type);

    for db_config in &db_configs {
        if !is_container_running(db_config.container_name) {
            println!("Starting {} container...", db_config.db_type);
            start_container(db_config);
            
            wait_for_container_ready(db_config);
        }
        
        for (format, filename) in &sample_files {
            println!("Testing {} with {}", db_config.db_type, filename);
            let args = Args {
                dump_file: Path::new(SAMPLE_DIR).join(filename).to_string_lossy().to_string(),
                vector_export_type: db_config.db_type.to_string(),
                vector_host: format!("{}:{}", db_config.host, db_config.port),
                database: TEST_DB_NAME.to_string(),
                tenant: "default_tenant".to_string(),
                namespace: "default_ns".to_string(),
                user: if db_config.db_type == "milvus" { "root" } else { "root" }.to_string(),
                pass: if db_config.db_type == "milvus" { "Milvus" } else { "root" }.to_string(),
                secret: "".to_string(),
                chunk_size: 10,
                dimension: 768,
                metric: "cosine".to_string(),

                embedding_model: std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| {
                    match provider_type {
                        TestEmbeddingProvider::Ollama => "nomic-embed-text".to_string(),
                        TestEmbeddingProvider::Tei => "nomic-ai/nomic-embed-text-v1.5".to_string(),
                        _ => "nomic-ai/nomic-embed-text-v1.5".to_string() 
                    }
                }),
                embedding_concurrency: 1,
                embedding_batch_size: 10,
                num_threads: 1,
                max_payload_size_mb: 1,
                tei_binary_path: "".to_string(),
                debug: true,
                use_auth: db_config.db_type != "redis",
                group_redis: false,
                use_exclude: false,
                indexes: "test_index".to_string(),
                cloud: "aws".to_string(),
                region: "us-east-1".to_string(),
                embedding_api_key: None,
                embedding_url: None,
                embedding_max_tokens: 8000,
                embedding_timeout: 60,
                embedding_task_type: "SEMANTIC_SIMILARITY".to_string(),
                tei_local_port: 19998,
                embedding_provider: match provider_type {
                    TestEmbeddingProvider::Ollama => "ollama".to_string(),
                    TestEmbeddingProvider::Tei => "tei".to_string(),
                    TestEmbeddingProvider::Google => "google".to_string(),
                    TestEmbeddingProvider::Mock => "mock".to_string(),
                },
            };
            
            let result = run_test_combination(&args, format, provider_type.clone());
            
            match result {
                Ok(_) => println!("✅ Test passed: {} with {} [Using: {:?}]", db_config.db_type, filename, provider_type),
                Err(err) => {
                    println!("❌ Test failed: {} with {} [Using: {:?}]", db_config.db_type, filename, provider_type);
                    println!("  Error: {}", err);
                }
            }
        }
    }
}

fn is_container_running(container_name: &str) -> bool {
    if container_name == "milvus-standalone-test" {
        let compose_file = Path::new(TESTS_DIR).join("docker-compose.yml");
        if !compose_file.exists() {
            return false;
        }
        
        let output = Command::new("docker-compose")
            .args(["-f", compose_file.to_str().unwrap(), "ps", "--services", "--filter", "status=running"])
            .output();
        
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains("standalone")
            },
            Err(_) => false,
        }
    } else {
        let output = Command::new("docker")
            .args(["ps", "--filter", &format!("name={}", container_name), "--format", "{{.Names}}"])
            .output();
            
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.trim().contains(container_name)
            },
            Err(_) => false,
        }
    }
}

fn start_container(config: &TestConfig) {
    if config.db_type == "milvus" {
        let compose_file = Path::new(TESTS_DIR).join("docker-compose.yml");
        if !compose_file.exists() {
            println!("Warning: docker-compose.yml not found for Milvus. Expected at: {}", compose_file.display());
            return;
        }
        
        println!("Starting Milvus cluster using docker-compose...");
        let status = Command::new("docker-compose")
            .args(["-f", compose_file.to_str().unwrap(), "up", "-d"])
            .status();
            
        match status {
            Ok(status) => {
                if !status.success() {
                    println!("Warning: docker-compose for Milvus may not have started properly");
                } else {
                    println!("Milvus cluster started successfully");
                }
            },
            Err(e) => {
                println!("Failed to start Milvus cluster: {}", e);
            }
        }
        
        return;
    }

    let parts: Vec<&str> = config.docker_cmd.split_whitespace().collect();
    
    let status = if parts.len() > 0 {
        Command::new(parts[0])
            .args(&parts[1..])
            .status()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(config.docker_cmd)
            .status()
    };
    
    match status {
        Ok(status) => {
            if !status.success() {
                println!("Warning: container {} may not have started properly", config.container_name);
            }
        },
        Err(e) => {
            println!("Failed to start container {}: {}", config.container_name, e);
        }
    }
}

fn wait_for_container_ready(config: &TestConfig) {
    println!("Waiting for {} to be ready...", config.db_type);
    if config.db_type == "milvus" {
        println!("Milvus cluster requires longer initialization time...");
        thread::sleep(Duration::from_secs(30));
    } else {
        thread::sleep(Duration::from_secs(10));
    }
    
    println!("{} should be ready", config.db_type);
}

fn run_test_combination(
    args: &Args,
    format: &str,
    provider_type: TestEmbeddingProvider,
) -> Result<(), String> {
    let sample_path = Path::new(&args.dump_file);
    
    let content = read_file_content(&sample_path)
        .map_err(|e| format!("Failed to read file {}: {}", args.dump_file, e))?;
    
    let parsed_records = parse_database_export(&content, format, args)
        .map_err(|e| format!("Failed to parse export: {}", e))?;
    
    if parsed_records.is_empty() {
        return Err("No records were parsed from the file".to_string());
    }
    
    
    use std::collections::HashMap;
    let mut table_groups: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    
    for record in parsed_records {
        if let Some(table) = record.get("table").and_then(|t| t.as_str()) {
            table_groups.entry(table.to_string())
                .or_insert_with(Vec::new)
                .push(record);
        }
    }

    let provider = create_embedding_provider(provider_type.clone(), args)
        .map_err(|e| format!("Failed to init provider: {}", e))?;
    let rt = Runtime::new().map_err(|e| e.to_string())?;

    let db = create_db_client(args).map_err(|e| format!("Failed to create DB client: {}", e))?;

    for (table, records) in table_groups {
        let texts: Vec<String> = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect();

        let embeddings = rt
            .block_on(provider.generate_embeddings_batch(&texts))
            .map_err(|e| format!("Embedding error: {}", e))?;

        let items: Vec<_> = records
            .into_iter()
            .enumerate()
            .map(|(i, record)| {
                let id = if args.vector_export_type == "qdrant" {
                    Uuid::new_v4().to_string()
                } else {
                    format!("mock-id-{}", i)
                };
                (id, embeddings[i].clone(), record)
            })
            .collect();

        db.store_vector(&table, &items)
            .map_err(|e| format!("Failed to store vectors: {}", e))?;
    }

    Ok(())
}

fn create_db_client(args: &Args) -> Result<Box<dyn Database>, String> {
    use db2vec::db::{
        chroma::ChromaDatabase,
        pinecone::PineconeDatabase,
        qdrant::QdrantDatabase,
        redis::RedisDatabase,
        milvus::MilvusDatabase,
        surreal::SurrealDatabase
    };
    
    match args.vector_export_type.as_str() {
        "chroma" => ChromaDatabase::new(args).map(|db| Box::new(db) as Box<dyn Database>),
        "pinecone" => PineconeDatabase::new(args).map(|db| Box::new(db) as Box<dyn Database>),
        "qdrant" => QdrantDatabase::new(args).map(|db| Box::new(db) as Box<dyn Database>),
        "redis" => RedisDatabase::new(args).map(|db| Box::new(db) as Box<dyn Database>),
        "milvus" => MilvusDatabase::new(args).map(|db| Box::new(db) as Box<dyn Database>),
        "surrealdb" => SurrealDatabase::new(args).map(|db| Box::new(db) as Box<dyn Database>),
        _ => Err(Box::<dyn std::error::Error + Send + Sync>::from(
            format!("Unsupported database type: {}", args.vector_export_type)
        )),
    }.map_err(|e| e.to_string())
}

