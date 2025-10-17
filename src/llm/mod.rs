use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use async_trait::async_trait;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

pub mod openai;

pub const ROLE_USER: &str = "user";
pub const ROLE_SYSTEM: &str = "system";
pub const ROLE_ASSISTANT: &str = "assistant";

pub struct LLMConversation {
    pub role: String,
    pub content: Arc<String>,
}

#[async_trait]
pub trait LLM: Send + Sync {
    async fn load_from_yaml(path: String) -> Result<Self, Box<dyn Error>> where Self: Sized;

    fn complete(&self, system_prompt: &String, conversation: &Vec<LLMConversation>) -> Pin<Box<dyn Stream<Item=Result<String, Box<dyn Error + Send>>> + Send>>;

    fn single_chat_stream(&self, prompt: Arc<String>) -> Pin<Box<dyn Stream<Item=Result<String, Box<dyn Error + Send>>> + Send>> {
        self.complete(&"".to_string(),
                      &vec!(LLMConversation{role: ROLE_USER.to_string(), content: prompt}))
    }

    async fn single_chat(&self, prompt: Arc<String>) -> Result<String, Box<dyn Error>> {
        let mut stream = self.single_chat_stream(prompt);
        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(s) => result.push_str(&s),
                Err(e) => return Err(e),
            }
        }
        Ok(result)
    }
}