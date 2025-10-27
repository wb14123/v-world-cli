use std::sync::Arc;
use log::info;
use tokio::sync::RwLock;
use tokio::sync::watch::Sender;

#[derive(Debug)]
pub struct ChatMessage {
    pub from_user_id: String,
    pub from_username: String,
    pub role: String,
    pub content_stream: Arc<Sender<(Arc<RwLock<Vec<String>>>, bool)>>,
}

impl ChatMessage {
    pub async fn read_content(&self) -> String {
        let mut sub = self.content_stream.subscribe();
        let mut final_content: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(vec![]));
        loop {
            {
                let result = sub.borrow_and_update();
                let (content, is_complete) = &*result;
                if *is_complete {
                    info!("finish is true {:?}", result);
                    final_content = content.clone();
                    break;
                }
            }
            let changed = sub.changed().await;
            if changed.is_err() {
                info!("changed {:?}", changed);
                break;
            }
        }
        final_content.read().await.join("").replace(&format!("{}(@{}): ", self.from_username, self.from_user_id), "")
    }
}

#[derive(Debug)]
pub struct ErrorMessage {
    pub msg: String,
}

#[derive(Clone, Debug)]
pub enum Message {
    Chat(Arc<ChatMessage>),
    Error(Arc<ErrorMessage>),
}