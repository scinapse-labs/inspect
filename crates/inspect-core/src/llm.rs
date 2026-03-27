use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::EntityReview;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityLlmReview {
    pub entity_name: String,
    pub file_path: String,
    pub verdict: LlmVerdict,
    pub issues: Vec<LlmIssue>,
    pub summary: String,
    pub tokens_used: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmVerdict {
    Approve,
    Comment,
    RequestChanges,
}

impl std::fmt::Display for LlmVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approve => write!(f, "approve"),
            Self::Comment => write!(f, "comment"),
            Self::RequestChanges => write!(f, "request_changes"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmIssue {
    pub severity: String,
    pub description: String,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn review_entity(&self, entity: &EntityReview) -> Result<EntityLlmReview, String>;
}

// --- Anthropic structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    usage: AnthropicUsage,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicContentBlock {
    text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

// --- OpenAI structs ---

#[derive(Debug, Clone, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

// --- Shared ---

#[derive(Debug, Clone, Deserialize)]
struct LlmOutput {
    verdict: LlmVerdict,
    #[serde(default)]
    issues: Vec<LlmIssue>,
    #[serde(default)]
    summary: String,
}

fn parse_llm_output(text: &str, entity: &EntityReview, tokens: u64) -> EntityLlmReview {
    let json_str = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(text)
        .trim();

    let output: LlmOutput = serde_json::from_str(json_str).unwrap_or(LlmOutput {
        verdict: LlmVerdict::Comment,
        issues: vec![LlmIssue {
            severity: "info".to_string(),
            description: text.to_string(),
        }],
        summary: "Could not parse structured response".to_string(),
    });

    EntityLlmReview {
        entity_name: entity.entity_name.clone(),
        file_path: entity.file_path.clone(),
        verdict: output.verdict,
        issues: output.issues,
        summary: output.summary,
        tokens_used: tokens,
    }
}

// --- AnthropicClient ---

pub struct AnthropicClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicClient {
    pub fn new(model: &str, api_key: Option<&str>) -> Result<Self, String> {
        let api_key = api_key
            .map(|k| k.to_string())
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok().filter(|k| !k.is_empty()))
            .ok_or_else(|| "ANTHROPIC_API_KEY not set. Set it to use LLM review.".to_string())?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.to_string(),
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicClient {
    async fn review_entity(&self, entity: &EntityReview) -> Result<EntityLlmReview, String> {
        let prompt = build_prompt(entity);

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            system: SYSTEM_PROMPT.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let api_resp: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        let text = api_resp
            .content
            .first()
            .and_then(|b| b.text.as_deref())
            .unwrap_or("");

        let tokens = api_resp.usage.input_tokens + api_resp.usage.output_tokens;

        Ok(parse_llm_output(text, entity, tokens))
    }
}

// --- OpenAIClient ---

pub struct OpenAIClient {
    client: reqwest::Client,
    api_key: Option<String>,
    api_base: String,
    model: String,
}

impl OpenAIClient {
    pub fn new(model: &str, api_base: Option<&str>, api_key: Option<&str>) -> Result<Self, String> {
        let api_key = api_key
            .map(|k| k.to_string())
            .or_else(|| std::env::var("OPENAI_API_KEY").ok().filter(|k| !k.is_empty()));

        let api_base = api_base
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            api_base,
            model: model.to_string(),
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAIClient {
    async fn review_entity(&self, entity: &EntityReview) -> Result<EntityLlmReview, String> {
        let prompt = build_prompt(entity);

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
            max_tokens: 1024,
        };

        let url = format!("{}/chat/completions", self.api_base);

        let mut req = self
            .client
            .post(&url)
            .header("content-type", "application/json");

        if let Some(ref key) = self.api_key {
            req = req.header("authorization", format!("Bearer {}", key));
        }

        let resp = req
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let api_resp: OpenAIResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        let text = api_resp
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");

        let tokens = api_resp
            .usage
            .map(|u| u.prompt_tokens + u.completion_tokens)
            .unwrap_or(0);

        Ok(parse_llm_output(text, entity, tokens))
    }
}

// --- Shared helpers ---

const SYSTEM_PROMPT: &str = "\
You are a code reviewer. Review the entity for bugs, security issues, and correctness problems. \
Respond with JSON only, no explanation outside the JSON. Format:
{\"verdict\": \"approve\" | \"comment\" | \"request_changes\", \"issues\": [{\"severity\": \"error\" | \"warning\" | \"info\", \"description\": \"...\"}], \"summary\": \"one sentence\"}";

fn build_prompt(entity: &EntityReview) -> String {
    let mut parts = vec![
        format!("Entity: {} ({})", entity.entity_name, entity.entity_type),
        format!("File: {}", entity.file_path),
        format!("Change: {:?}", entity.change_type),
        format!("Classification: {}", entity.classification),
        format!("Risk: {} (score {:.2})", entity.risk_level, entity.risk_score),
        format!("Blast radius: {}, Dependents: {}", entity.blast_radius, entity.dependent_count),
    ];

    if entity.is_public_api {
        parts.push("Public API: yes".to_string());
    }

    if !entity.dependent_names.is_empty() {
        let deps: Vec<String> = entity
            .dependent_names
            .iter()
            .take(10)
            .map(|(name, file)| format!("  {} ({})", name, file))
            .collect();
        parts.push(format!("Dependents:\n{}", deps.join("\n")));
    }

    if let Some(ref before) = entity.before_content {
        parts.push(format!("BEFORE:\n```\n{}\n```", before));
    }

    if let Some(ref after) = entity.after_content {
        parts.push(format!("AFTER:\n```\n{}\n```", after));
    }

    parts.join("\n\n")
}
