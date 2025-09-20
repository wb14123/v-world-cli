use std::error::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};

/// A bot profile containing personal information and conversation examples.
#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Profile {
    /// The user's display name or username
    name: String,
    
    /// A brief description of the user's background, experience, or context
    background: String,

    /// Sample conversations or phrases that represent the user's communication style
    conversation_examples: Vec<String>,

    llm_provider: String,
    llm_model: String,
}

pub async fn new(yaml_file: &String) -> Result<Profile, Box<dyn Error>> {
    let mut f = File::open(yaml_file).await?;
    let mut contents = String::new();
    f.read_to_string(&mut contents).await?;
    Ok(serde_yaml::from_str(&contents)?)
}

pub async fn create_template_file(output_path: &String) -> Result<(), Box<dyn Error>> {
    let mut f = File::create(output_path).await?;
    let profile = Profile::default();
    let yaml_content = serde_yaml::to_string(&profile)?;
    f.write_all(yaml_content.as_bytes()).await?;
    Ok(())
}
