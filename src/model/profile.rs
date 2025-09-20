use std::error::Error;
use std::fs::File;
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

pub fn new(yaml_file: &String) -> Result<Profile, Box<dyn Error>> {
    let f = File::open(yaml_file)?;
    Ok(serde_yaml::from_reader(f)?)
}

pub fn create_template_file(output_path: &String) -> Result<(), Box<dyn Error>> {
    let f = File::create(output_path)?;
    let profile = Profile::default();
    Ok(serde_yaml::to_writer(f, &profile)?)
}
