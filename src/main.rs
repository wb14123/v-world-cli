use std::sync::Arc;
use clap::{Parser, Subcommand};
use crate::dao::profile_dao::ProfileDao;
use crate::model::profile::Profile;
use tokio_stream::{self as stream, StreamExt};
use crate::chat::plan_agent::PlanAgent;
use crate::chat::room::Room;
use crate::llm::LLM;
use crate::llm::openai::OpenAI;
use crate::ui::cli_ui::CliUI;

mod model;
mod dao;
mod chat;
mod llm;
mod ui;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    profile_path: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    CreateProfile {
        #[arg(short, long)]
        id: String,
    },
    NewChat {
        #[arg(short, long)]
        profile_ids: Vec<String>,
        #[arg(short, long)]
        llm_config: String,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    log4rs::init_file("log4rs.yaml", Default::default())?;

    let cli = Cli::parse();
    let profile_dao = Arc::new(dao::profile_yaml_dao::new(cli.profile_path).await?);
    match cli.command {
        Commands::CreateProfile { id} => {
            let mut p = Profile::default();
            p.id = id;
            let created = profile_dao.create(&p).await?;
            if created {
                println!("Profile template file created successfully");
            } else {
                println!("Profile template file already exists");
            }
        }
        Commands::NewChat {profile_ids, llm_config} => {
            let llm = OpenAI::load_from_yaml(llm_config).await?;
            let profiles: Vec<Arc<Profile>> = stream::iter(profile_ids)
                .then(|id| {
                    let dao = profile_dao.clone();
                    async move { dao.get(&id).await }
                })
                .filter_map(|result| {
                    match result {
                        Ok(Some(p)) => Some(Arc::new(p)),
                        Ok(None) => None,
                        Err(e) => {
                            eprintln!("Error when get profile: {}", e);
                            None
                        }
                    }
                })
                .collect()
                .await;
            let room = Arc::new(Room::new(100, profiles));
            let plan_agent = Arc::new(PlanAgent::new(Arc::new(llm), room.clone()));
            plan_agent.start().await;
            let ui = CliUI::new(room.clone(), Arc::new("tuser".into()), Arc::new("Test User".into()));
            ui.start()?
        }
    }
    Ok(())
}
