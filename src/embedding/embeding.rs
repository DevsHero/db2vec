use log::{ error, info, warn };
use reqwest::blocking::Client as HttpClient;
use std::env;

pub fn generate_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let http_client = HttpClient::builder().timeout(std::time::Duration::from_secs(30)).build()?;
    let trimmed_text = if text.len() > 8000 { &text[0..8000] } else { text };
    let max_retries = 3;
    let mut retry_count = 0;

    let embedding_url = env
        ::var("EMBEDDING_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/embeddings".to_string());
    let embedding_model = env
        ::var("EMBEDDING_MODEL")
        .unwrap_or_else(|_| "nomic-embed-text".to_string());

    while retry_count < max_retries {
        let response = http_client
            .post(&embedding_url)
            .header("Content-Type", "application/json")
            .json(
                &serde_json::json!({
                    "model": embedding_model,
                    "prompt": trimmed_text
                })
            )
            .send()?;
        if !response.status().is_success() {
            warn!("Embedding API returned status: {}", response.status());
            retry_count += 1;
            std::thread::sleep(std::time::Duration::from_secs(2));
            continue;
        }

        let json = response.json::<serde_json::Value>()?;
        if let Some(embedding_array) = json["embedding"].as_array() {
            let embedding: Vec<f32> = embedding_array
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();

            info!("Generated embedding with {} dimensions", embedding.len());
            return Ok(embedding);
        } else {
            error!("Unexpected response structure: {:?}", json);
            retry_count += 1;
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }

    Err("Failed to get embeddings after retries".into())
}
