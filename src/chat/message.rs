use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub from_user_id: String,
    pub role: String,
    pub content: Arc<String>,
}

#[derive(Clone, Debug)]
pub struct ErrorMessage {
    pub msg: String,
}

#[derive(Clone, Debug)]
pub enum Message {
    Chat(Arc<ChatMessage>),
    Error(Arc<ErrorMessage>),
}