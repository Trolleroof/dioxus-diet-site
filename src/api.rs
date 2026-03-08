use crate::models::DietAnalysis;
use dioxus::prelude::*;
use serde_json::Value;
use std::sync::OnceLock;

static ENV_LOADED: OnceLock<()> = OnceLock::new();

fn ensure_env_loaded() {
    ENV_LOADED.get_or_init(|| {
        dotenvy::dotenv().ok();
    });
}

const SYSTEM_PROMPT: &str = r#"You are a food dietary analysis assistant. Analyze the food shown in the image and return a JSON object with these exact fields:

{
  "food_name": "Name of the food/dish",
  "description": "Brief description of what you see",
  "dietary_tags": ["Vegetarian", "Vegan", "Gluten-Free", "Dairy-Free", "Nut-Free", "Halal", "Kosher", "Contains Meat", "Contains Dairy", "Contains Gluten", "Contains Eggs"],
  "allergens": ["List any common allergens: milk, eggs, fish, shellfish, tree nuts, peanuts, wheat, soy, sesame"],
  "macros_estimate": {
    "calories": "estimated calories e.g. '350-450'",
    "protein": "estimated grams e.g. '15-20g'",
    "carbs": "estimated grams e.g. '40-50g'",
    "fat": "estimated grams e.g. '10-15g'",
    "fiber": "estimated grams e.g. '3-5g'"
  },
  "ingredients_detected": ["List visible/likely ingredients"],
  "confidence": "high, medium, or low",
  "notes": "Any important dietary notes, warnings, or observations"
}

Only include dietary_tags that apply. Only include allergens that are likely present. Return ONLY the JSON object, no markdown fences or extra text."#;

// Server-side function that calls OpenRouter API
#[server]
pub async fn analyze_food(image_base64: String) -> Result<DietAnalysis, ServerFnError> {
    // Ensure .env file is loaded
    ensure_env_loaded();
    
    // Get API key from environment variable
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| ServerFnError::new("OPENROUTER_API_KEY environment variable not set"))?;

    // Use free vision model
    let model = "mistralai/mistral-small-3.1-24b-instruct:free";

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/jpeg;base64,{}", image_base64)
                        }
                    },
                    {
                        "type": "text",
                        "text": "Analyze this food image for dietary information."
                    }
                ]
            }
        ],
        "max_tokens": 1024,
        "temperature": 0.3
    });

    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Network error: {}", e)))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to read response: {}", e)))?;

    if !status.is_success() {
        return Err(ServerFnError::new(format!("API error ({}): {}", status, text)));
    }

    let json: Value = serde_json::from_str(&text)
        .map_err(|e| ServerFnError::new(format!("Invalid JSON response: {}", e)))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| ServerFnError::new("No content in API response".to_string()))?;

    // Strip markdown code fences if present
    let clean = content.trim();
    let clean = if clean.starts_with("```") {
        let start = clean.find('\n').map(|i| i + 1).unwrap_or(0);
        let end = clean.rfind("```").unwrap_or(clean.len());
        &clean[start..end]
    } else {
        clean
    };

    serde_json::from_str(clean.trim())
        .map_err(|e| ServerFnError::new(format!("Failed to parse analysis: {}. Raw: {}", e, clean)))
}

// Server-side function for answering questions about the food image
#[server]
pub async fn ask_question(image_base64: String, question: String) -> Result<String, ServerFnError> {
    // Validate inputs
    if image_base64.is_empty() {
        return Err(ServerFnError::new("Image is required to ask questions"));
    }
    if question.trim().is_empty() {
        return Err(ServerFnError::new("Question cannot be empty"));
    }
    
    // Ensure .env file is loaded
    ensure_env_loaded();
    
    // Get API key from environment variable
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| ServerFnError::new("OPENROUTER_API_KEY environment variable not set"))?;

    // Use free vision model
    let model = "mistralai/mistral-small-3.1-24b-instruct:free";

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful food dietary analysis assistant. Answer questions about food images clearly and concisely. Be specific and accurate."
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/jpeg;base64,{}", image_base64)
                        }
                    },
                    {
                        "type": "text",
                        "text": question
                    }
                ]
            }
        ],
        "max_tokens": 512,
        "temperature": 0.7
    });

    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Network error: {}", e)))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to read response: {}", e)))?;

    if !status.is_success() {
        return Err(ServerFnError::new(format!("API error ({}): {}", status, text)));
    }

    let json: Value = serde_json::from_str(&text)
        .map_err(|e| ServerFnError::new(format!("Invalid JSON response: {}", e)))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| ServerFnError::new("No content in API response".to_string()))?;

    Ok(content.trim().to_string())
}

/// Transcribe audio via Groq (free) or OpenAI Whisper.
/// Set GROQ_API_KEY in .env for free tier, or OPENAI_API_KEY for OpenAI.
#[server]
pub async fn transcribe_audio(audio_base64: String, mime_type: String) -> Result<String, ServerFnError> {
    use base64::Engine as _;

    ensure_env_loaded();

    let (api_key, endpoint, model) =
        if let Ok(k) = std::env::var("GROQ_API_KEY") {
            (k, "https://api.groq.com/openai/v1/audio/transcriptions", "whisper-large-v3-turbo")
        } else if let Ok(k) = std::env::var("OPENAI_API_KEY") {
            (k, "https://api.openai.com/v1/audio/transcriptions", "whisper-1")
        } else {
            return Err(ServerFnError::new(
                "Voice transcription requires GROQ_API_KEY (free) or OPENAI_API_KEY in .env"
            ));
        };

    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(&audio_base64)
        .map_err(|e| ServerFnError::new(format!("Failed to decode audio: {}", e)))?;

    let ext = match mime_type.as_str() {
        t if t.contains("mp4") || t.contains("m4a") => "m4a",
        t if t.contains("webm") => "webm",
        t if t.contains("ogg") => "ogg",
        t if t.contains("wav") => "wav",
        _ => "mp4",
    };

    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", ext))
        .mime_str(&mime_type)
        .map_err(|e| ServerFnError::new(format!("Mime error: {}", e)))?;

    let form = reqwest::multipart::Form::new()
        .text("model", model)
        .part("file", part);

    let client = reqwest::Client::new();
    let response = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Network error: {}", e)))?;

    let status = response.status();
    let text = response.text().await
        .map_err(|e| ServerFnError::new(format!("Response error: {}", e)))?;

    if !status.is_success() {
        return Err(ServerFnError::new(format!("Transcription failed ({}): {}", status, text)));
    }

    let json: Value = serde_json::from_str(&text)
        .map_err(|e| ServerFnError::new(format!("JSON error: {}", e)))?;

    Ok(json["text"].as_str().unwrap_or("").trim().to_string())
}
