use serde::{Deserialize, Serialize};

/// A bot profile containing personal information and conversation examples.
#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Profile {
    pub id: String,

    /// The user's display name or username
    pub name: String,
    
    /// A brief description of the user's background, experience, or context
    pub background: String,

    /// Sample conversations or phrases that represent the user's communication style
    pub conversation_examples: Vec<String>,

    pub llm_provider: String,
    pub llm_model: String,
}
