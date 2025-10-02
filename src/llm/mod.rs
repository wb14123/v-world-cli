use std::error::Error;
use std::sync::Arc;
use async_trait::async_trait;

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

    async fn complete(&self, system_prompt: &String, conversation: &Vec<LLMConversation>) -> Result<String, Box<dyn Error>>;

    async fn single_chat(&self, prompt: Arc<String>) -> Result<String, Box<dyn Error>> {
        self.complete(&"".to_string(),
                      &vec!(LLMConversation{role: "User".to_string(), content: prompt})).await
    }
}