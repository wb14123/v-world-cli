use std::error::Error;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;
use tokio::sync::{watch, RwLock};
use tokio::task::JoinHandle;
use log::info;
use tokio_stream::StreamExt;
use crate::chat::message::{ChatMessage, ErrorMessage, Message};
use crate::chat::room::Room;
use crate::llm::{LLMConversation, LLM, ROLE_ASSISTANT};
use crate::model::profile::Profile;

pub struct PlanAgent {
    llm: Arc<dyn LLM>,
    room: Arc<Room>,
    msg_receiver: Receiver<Message>,
    recent_chats: Vec<Arc<ChatMessage>>,
    profiles_summarize: String,
}

impl PlanAgent {
    pub fn new(llm: Arc<dyn LLM>, room: Arc<Room>) -> Self {
        PlanAgent{
            llm,
            room: room.clone(),
            msg_receiver: room.subscribe(),
            recent_chats: Vec::new(),
            profiles_summarize: Self::summarize_profile(&room.profiles)
        }
    }

    pub async fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let result = self.loop_worker().await;
                match result {
                    Ok(..) => info!("Handled message in plan agent."),
                    Err(err) => {
                        self.room
                            .send_error(Arc::new(ErrorMessage { msg: format!("Failed to handle message: {}", err) }))
                            .expect("cannot send error msg");
                    }
                }
            }
        })
    }

    async fn loop_worker(&mut self) -> Result<(), Box<dyn Error>> {
        let msg = self.msg_receiver.recv().await?;
        match msg {
            Message::Chat(chat) => {
                info!("received chat: {:?}", chat);
                Ok(self.on_chat(chat).await?)
            }
            _ => Ok(())
        }
    }

    fn summarize_profile(profiles: &Vec<Arc<Profile>>) -> String {
        profiles.iter()
            .map(|p| format!("ID: {}\nName: {}\nBackground: {}", p.id, p.name, p.background))
            .collect::<Vec<String>>().join("\n--------------")
    }

    async fn get_prompt(profile_summary: &String, recent_messages: &Vec<Arc<ChatMessage>>) -> String {
        let mut recent_msg_vec = Vec::new();
        for m in recent_messages.iter() {
            recent_msg_vec.push(format!("{}(@{}): {}", m.from_username, m.from_user_id, m.read_content().await));
        }
        let recent_msg_str = recent_msg_vec.join("\n");
        format!("You are given a summary of profiles for all the LLM agent in the conversation.\
        You are also given the recent conversation of the agents and the user. Based on that, \
        output which LLM agent should reply in the conversation next. The output can be either of the two:\n\
        \n\
        * Agent ID with a prefix of @. This indicates which agent should reply next.\n\
        * Simply response `no reply` that indicates no agent should reply to the conversation next.\n\
        * Only select the profile from the profile summary. The recent conversations also contain the real user IDs that you shouldn't select from.
        \n\
        Follow the output format strictly and output nothing else.\n\
        If the last message is sent by the user, there always should have an agent to reply.\
        Otherwise it's optional for other agents to reply.\n\
        Here are the agent profile summary: \n\
        {profile_summary}
        Here are the recent conversations: \n\
        {recent_msg_str}
        ")
    }

    async fn on_chat(&mut self, msg: Arc<ChatMessage>) -> Result<(), Box<dyn Error>> {
        self.recent_chats.push(msg);
        let prompt = Self::get_prompt(&self.profiles_summarize, &self.recent_chats).await;
        let next_user = self.llm.single_chat(Arc::new(prompt)).await?;
        if next_user.starts_with("@") {
            let next_id = next_user.trim_start_matches("@").to_string();
            match self.room.profiles.iter().find(|p| p.id == next_id) {
                Some(profile) => Ok(self.complete_chat(profile).await?),
                None => Err(format!("No profile found for id {}", next_id).into()),
            }
        } else if next_user.eq("no reply") {
            info!("No reply needed from plan agent.");
            Ok(())
        } else {
            Err(format!("Got unexpected result from plan agent: {}", next_user).into())
        }
    }

    async fn complete_chat(&self, profile: &Profile) -> Result<(), Box<dyn Error>> {
        // TODO: include profile conversation examples
        let system_prompt = format!("You are simulating a profile in a group chat to reply a new message. \
            You must reply the message.\n\
            Here is the background of the profile: \n\
            id: {}\n\
            name: {}\n\
            background:\n{}\
            ", profile.id, profile.name, profile.background);
        let mut conversation = Vec::new();
        for m in self.recent_chats.iter() {
            conversation.push(LLMConversation{
                role: m.role.clone(),
                content: Arc::new(format!("{}(@{}): {}", m.from_username, m.from_user_id, m.read_content().await)),
            });
        }
        let content_vec = Arc::new(RwLock::new(vec![]));
        let (sender, _rx) = watch::channel((content_vec.clone(), false));
        let sender_ref = Arc::new(sender);
        let msg = ChatMessage{
            from_user_id: profile.id.clone(),
            from_username: profile.name.clone(),
            role: ROLE_ASSISTANT.to_string(),
            content_stream: sender_ref.clone(),
        };
        self.room.send_chat(Arc::new(msg))?;
        let mut stream = self.llm.complete(&system_prompt, &conversation);
        while let Some(response) = stream.next().await {
            let parsed_res = response.map_err(|e| e as Box<dyn Error>)?.replace(&format!("{}(@{}): ", profile.name, profile.id), "");
            content_vec.write().await.push(parsed_res);
            sender_ref.send((content_vec.clone(), false))?;
        };
        sender_ref.send((content_vec, true))?;
        Ok(())
    }
}
