use std::error::Error;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;
use crate::chat::message::{ChatMessage, Message};
use crate::model::profile::Profile;

pub struct Room {
    pub profiles: Vec<Arc<Profile>>,
    sender: Sender<Message>,
}

impl Room {
    pub fn new(channel_size: usize) -> Self {
        let (tx, rx) = broadcast::channel(channel_size);
        Room { sender: tx , profiles: Vec::new() }
    }

    pub fn send_chat(&self, msg: Arc<ChatMessage>) -> Result<(), Box<dyn Error>> {
        self.sender.send(Message::Chat(msg))?;
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Message> {
        self.sender.subscribe()
    }
}