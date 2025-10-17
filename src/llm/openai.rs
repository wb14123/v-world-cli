use super::{LLMConversation, LLM, ROLE_SYSTEM};
use async_trait::async_trait;
use futures::stream::StreamExt;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::pin::Pin;
use tokio_stream::Stream;

#[derive(Debug, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_base_url() -> String {
    "https://api.openai.com/v1".to_string()
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
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

#[async_trait]
impl LLM for OpenAI {
    async fn load_from_yaml(path: String) -> Result<Self, Box<dyn Error>> {
        let content = tokio::fs::read_to_string(&path).await?;
        let config: OpenAIConfig = serde_yaml::from_str(&content)?;

        let client = reqwest::Client::new();

        Ok(OpenAI { config, client })
    }

    fn complete(&self, system_prompt: &String, conversation: &Vec<LLMConversation>) -> Pin<Box<dyn Stream<Item=Result<String, Box<dyn Error + Send>>> + Send>> {
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
            stream: true,
        };

        // Log the request
        info!("OpenAI API Request to model: {}", self.config.model);
        debug!("Request payload: {:?}", request);

        let url = format!("{}/chat/completions", self.config.base_url);
        let client = self.client.clone();
        let api_key = self.config.api_key.clone();

        let stream = async_stream::stream! {
            let response = match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await {
                    Ok(resp) => resp,
                    Err(e) => {
                        yield Err(Box::new(e) as Box<dyn Error + Send>);
                        return;
                    }
                };

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                info!("OpenAI API Error: status={}, error={}", status, error_text);
                yield Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("OpenAI API error: {}", error_text))) as Box<dyn Error + Send>);
                return;
            }

            let mut stream = response.bytes_stream();

            while let Some(result) = stream.next().await {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).to_string();
                        debug!("Response payload: {:?}", text);

                        let content = text.lines()
                            .filter(|line| line.starts_with("data: "))
                            .filter_map(|line| {
                                let json_str = line.strip_prefix("data: ")?;

                                // Skip the [DONE] message
                                if json_str.trim() == "[DONE]" {
                                    return None;
                                }

                                // Parse the JSON chunk
                                serde_json::from_str::<ChatCompletionResponse>(json_str)
                                    .ok()
                                    .and_then(|chunk| {
                                        chunk.choices.first().and_then(|choice| {
                                            choice.delta.as_ref().and_then(|delta| {
                                                delta.content.clone()
                                            })
                                        })
                                    })
                            })
                            .collect::<Vec<String>>().join("");

                        if !content.is_empty() {
                            yield Ok(content);
                        }
                    }
                    Err(e) => {
                        yield Err(Box::new(e) as Box<dyn Error + Send>);
                        return;
                    }
                }
            }
        };

        Box::pin(stream)
    }
}
