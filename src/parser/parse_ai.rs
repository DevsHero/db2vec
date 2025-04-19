use serde_json::Value;
use reqwest::blocking::Client as HttpClient;

use crate::parser::parse_regex::extract_json_array;

pub fn parse_with_ai(
    chunk_text: &str,
    format: &str,
    chunk_index: usize
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let http_client = HttpClient::builder().timeout(std::time::Duration::from_secs(300)).build()?;

    let chunk_size = 50000;
    let clean_text = chunk_text.to_string();

    let prompt = format!(
        r#"You are a strict JSON extractor for database exports.
- Only extract objects from inside INSERT [ ... ]; blocks.
- Ignore all schema, comments, and non-data lines.
- For any field value that contains HTML tags, extract only the readable text (strip all tags).
- Output a JSON array of objects, one per record, with all fields as key-value pairs and HTML fields as plain text.
- The response must start with '[' and end with ']'. If you can't parse, return [].

Database format: {}
```
{}
```"#,
        format,
        &clean_text[0..chunk_size.min(clean_text.len())]
    );
    let models = ["cogito:8b"];
    let mut json_str = String::new();

    for model in models {
        println!("Trying model {} for chunk {}", model, chunk_index);
        let response = http_client
            .post("http://localhost:11434/api/generate")
            .json(
                &serde_json::json!({
                "model": model,
                "prompt": prompt,
                "stream": false,
                "temperature": 0.1,
                "system": "You are a JSON formatter. Output only valid JSON arrays. Extract key info from HTML as text."
            })
            )
            .send()?;

        println!(
            "Chunk {} parsing with model {} status: {}",
            chunk_index,
            model,
            response.status()
        );

        let json_response = response.json::<serde_json::Value>()?;

        if let Some(text) = json_response["response"].as_str() {
            println!("AI raw response for chunk {}:\n{}", chunk_index, text);
            let fixed_text = fix_json_text(text);
            match serde_json::from_str::<Vec<Value>>(&fixed_text) {
                Ok(mut chunk_records) => {
                    if !chunk_records.is_empty() {
                        for record in chunk_records.iter_mut() {
                            if let Some(obj) = record.as_object_mut() {
                                obj.remove("id");
                                for (_k, v) in obj.iter_mut() {
                                    if let Some(s) = v.as_str() {
                                        if s.contains('<') && s.contains('>') {
                                            *v = Value::String(extract_text_from_html(s));
                                        }
                                    }
                                }
                            }
                        }
                        println!(
                            "Parsed {} records from chunk {} with model {}",
                            chunk_records.len(),
                            chunk_index,
                            model
                        );
                        return Ok(chunk_records);
                    } else {
                        println!("Model {} returned empty array, trying next model", model);
                    }
                }
                Err(e) => {
                    println!("Error parsing JSON from model {}: {}", model, e);
                    json_str = fixed_text;
                }
            }
        }
    }

    println!("All models failed for chunk {}, trying manual extraction", chunk_index);

    if !json_str.is_empty() {
        println!("JSON text sample: {}", &json_str[0..(100).min(json_str.len())]);

        if let Some(json_array) = extract_json_array(&json_str) {
            match serde_json::from_str::<Vec<Value>>(json_array) {
                Ok(mut chunk_records) => {
                    if !chunk_records.is_empty() {
                        for record in chunk_records.iter_mut() {
                            if let Some(obj) = record.as_object_mut() {
                                obj.remove("id");
                                for (_k, v) in obj.iter_mut() {
                                    if let Some(s) = v.as_str() {
                                        if s.contains('<') && s.contains('>') {
                                            *v = Value::String(extract_text_from_html(s));
                                        }
                                    }
                                }
                            }
                        }
                        println!("Manual extraction found {} records", chunk_records.len());
                        return Ok(chunk_records);
                    }
                }
                Err(_) => {}
            }
        }
    }

    println!("Could not parse any records from chunk {}", chunk_index);
    Ok(Vec::new())
}

fn fix_json_text(text: &str) -> String {
    if let (Some(start), Some(end)) = (text.find('['), text.rfind(']')) {
        if start < end {
            return text[start..=end].to_string();
        }
    }

    if !text.trim().starts_with('[') {
        if let Some(idx) = text.find('[') {
            let mut depth = 0;
            let mut in_string = false;
            let mut escape_next = false;

            for (i, c) in text[idx..].chars().enumerate() {
                match c {
                    '[' if !in_string => {
                        depth += 1;
                    }
                    ']' if !in_string => {
                        depth -= 1;
                        if depth == 0 {
                            return text[idx..=idx + i].to_string();
                        }
                    }
                    '"' if !escape_next => {
                        in_string = !in_string;
                    }
                    '\\' if in_string && !escape_next => {
                        escape_next = true;
                    }
                    _ => {
                        escape_next = false;
                    }
                }
            }
        }
    }

    if !text.trim().starts_with('[') && !text.trim().starts_with('{') {
        return "[]".to_string();
    }

    text.to_string()
}
fn extract_text_from_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        match c {
            '<' => {
                in_tag = true;
            }
            '>' => {
                in_tag = false;
            }
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    let space_regex = regex::Regex::new(r"\s+").unwrap();
    space_regex.replace_all(&result, " ").to_string()
}
