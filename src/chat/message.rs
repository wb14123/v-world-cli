use std::sync::Arc;
use log::info;
use tokio::sync::watch::Sender;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub from_user_id: String,
    pub from_username: String,
    pub role: String,
    pub content_stream: Arc<Sender<(Arc<Vec<String>>, bool)>>,
}

impl ChatMessage {
    pub async fn read_content(&self) -> String {
        let mut sub = self.content_stream.subscribe();
        loop {
            {
                let result = sub.borrow_and_update();
                let (content, is_complete) = &*result;
                if *is_complete {
                    info!("finish is true {:?}", result);
                    return content.join("").replace(&format!("{}(@{}): ", self.from_username, self.from_user_id), "");
                }
            }
            let changed = sub.changed().await;
            if changed.is_err() {
                info!("changed {:?}", changed);
                break;
            }
        }
        String::new()
    }
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