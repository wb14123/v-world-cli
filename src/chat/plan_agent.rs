use std::error::Error;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use log::info;
use crate::chat::message::{ChatMessage, ErrorMessage, Message};
use crate::chat::room::Room;
use crate::llm::{LLMConversation, LLM, ROLE_ASSISTANT};
use crate::model::profile::Profile;

pub struct PlanAgent {
    llm: Arc<dyn LLM>,
    room: Arc<Room>,
    msg_receiver: RwLock<Receiver<Message>>,
    process: RwLock<Option<JoinHandle<()>>>,
    recent_chats: RwLock<Vec<Arc<ChatMessage>>>,
    profiles_summarize: String,
}

impl PlanAgent {
    pub fn new(llm: Arc<dyn LLM>, room: Arc<Room>) -> Self {
        PlanAgent{
            llm,
            room: room.clone(),
            msg_receiver: RwLock::new(room.subscribe()),
            process: RwLock::new(None),
            recent_chats: RwLock::new(Vec::new()),
            profiles_summarize: Self::summarize_profile(&room.profiles)
        }
    }

    pub async fn start(self: Arc<Self>) {
        if self.process.read().await.is_some() {
            return
        }
        let mut p = self.process.write().await;
        let self_clone = self.clone();
        *p = Some(tokio::spawn(async move {
            loop {
                let result = self_clone.loop_worker().await;
                match result {
                    Ok(..) => info!("Handled message in plan agent."),
                    Err(err) => {
                        self_clone.room
                            .send_error(Arc::new(ErrorMessage { msg: format!("Failed to handle message: {}", err) }))
                            .expect("cannot send error msg");
                    }
                }
            }
        }));
    }

    async fn loop_worker(&self) -> Result<(), Box<dyn Error>> {
        let msg = self.msg_receiver.write().await.recv().await?;
        match msg {
            Message::Chat(chat) => {
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

    fn get_prompt(profile_summary: &String, recent_messages: &Vec<Arc<ChatMessage>>) -> String {
        let recent_msg_str = recent_messages.iter()
            .map(|m| format!("@{}: {}", m.from_user_id, m.content))
            .collect::<Vec<String>>()
            .join("\n");
        format!("You are given a summary of profiles for all the LLM agent in the conversation.\
        You are also given the recent conversation of the agents and the user. Based on that, \
        output which LLM agent should reply in the conversation next. The output can be either of the two:\n\
        \n\
        * Agent ID with a prefix of @. This indicates which agent should reply next.\n\
        * Simply response `no reply` that indicates no agent should reply to the conversation next.\n\
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

    async fn on_chat(&self, msg: Arc<ChatMessage>) -> Result<(), Box<dyn Error>> {
        self.recent_chats.write().await.push(msg);
        let recent_chats = self.recent_chats.read().await;
        let prompt = Self::get_prompt(&self.profiles_summarize, &recent_chats);
        let result = self.llm.single_chat(Arc::new(prompt)).await?;
        if result.starts_with("@") {
            let next_id = result.trim_start_matches("@").to_string();
            match self.room.profiles.iter().find(|p| p.id == next_id) {
                Some(profile) => Ok(self.complete_chat(profile).await?),
                None => Err(format!("No profile found for id {}", next_id).into()),
            }
        } else if result.eq("no reply") {
            info!("No reply needed from plan agent.");
            Ok(())
        } else {
            Err(format!("Got unexpected result from plan agent: {}", result).into())
        }
    }

    async fn complete_chat(&self, profile: &Profile) -> Result<(), Box<dyn Error>> {
        // TODO: include profile conversation examples
        let system_prompt = format!("You are simulating a profile in a group chat to reply a new message. \
            Here is the background of the profile: \n\
            id: {}\n\
            name: {}\n\
            background:\n{}\
            ", profile.id, profile.name, profile.background);
        let recent_chats = self.recent_chats.read().await;
        let conversation = recent_chats.iter()
            .map(|m| LLMConversation{
                role: m.role.clone(),
                content: Arc::new(format!("{}(@{}): {}", m.from_username, m.from_user_id, m.content)),
            })
            .collect::<Vec<LLMConversation>>();
        let response = self.llm.complete(&system_prompt, &conversation).await?;
        let parsed_res = response.replace(&format!("{}(@{}): ", profile.name, profile.id), "");
        self.room.send_chat(Arc::new(ChatMessage{
                from_user_id: profile.id.clone(),
                from_username: profile.name.clone(),
                role: ROLE_ASSISTANT.to_string(),
                content: Arc::new(parsed_res),
            }))?;
        Ok(())
    }
}
