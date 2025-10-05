use super::{LLMConversation, LLM, ROLE_SYSTEM};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    2048
}

pub struct OpenAI {
    config: OpenAIConfig,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    content: String,
}

#[async_trait]
impl LLM for OpenAI {
    async fn load_from_yaml(path: String) -> Result<Self, Box<dyn Error>> {
        let content = tokio::fs::read_to_string(&path).await?;
        let config: OpenAIConfig = serde_yaml::from_str(&content)?;

        let client = reqwest::Client::new();

        Ok(OpenAI { config, client })
    }

    async fn complete(&self, system_prompt: &String, conversation: &Vec<LLMConversation>) -> Result<String, Box<dyn Error>> {
        let mut messages = Vec::new();

        // Add system prompt if provided
        if !system_prompt.is_empty() {
            messages.push(ChatMessage {
                role: ROLE_SYSTEM.to_string(),
                content: system_prompt.clone(),
            });
        }

        // Add conversation history
        for conv in conversation {
            messages.push(ChatMessage {
                role: conv.role.clone(),
                content: conv.content.as_ref().clone(),
            });
        }

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        };

        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("OpenAI API error: {}", error_text).into());
        }

        let completion: ChatCompletionResponse = response.json().await?;

        completion.choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| "No response from OpenAI".into())
    }
}
