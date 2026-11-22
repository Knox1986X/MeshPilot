//! LLM: Groq / GigaChat / Hermes (Ollama) через HTTP (reqwest).
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatResult {
    pub content: String,
    pub provider: String,
}

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

#[tauri::command]
pub async fn chat(messages: Vec<ChatMessage>) -> Result<ChatResult, String> {
    // 1. Groq
    let groq_key = env("GROQ_API_KEY");
    if !groq_key.is_empty() {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": "llama-3.3-70b-versatile",
            "messages": messages.iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
        });
        let resp = client.post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {groq_key}"))
            .json(&body).send().await.map_err(|e| e.to_string())?;
        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
        return Ok(ChatResult { content, provider: "groq".into() });
    }
    // 2. Hermes (Ollama локально)
    let hermes_url = env("HERMES_BASE_URL");
    let hermes_url = if hermes_url.is_empty() { "http://localhost:11434/v1".to_string() } else { hermes_url };
    let hermes_model = env("HERMES_MODEL");
    let hermes_model = if hermes_model.is_empty() { "nous-hermes3:8b".to_string() } else { hermes_model };
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": hermes_model,
        "messages": messages.iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
    });
    let resp = client.post(format!("{}/chat/completions", hermes_url.trim_end_matches('/')))
        .json(&body).send().await.map_err(|e| format!("Hermes недоступен (ollama serve?): {e}"))?;
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
    Ok(ChatResult { content, provider: "hermes".into() })
}
