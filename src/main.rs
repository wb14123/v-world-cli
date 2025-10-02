
use clap::{Parser, Subcommand};
use crate::dao::profile_dao::ProfileDao;
use crate::model::profile::Profile;

mod model;
mod dao;
mod chat;
mod llm;

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
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let profile_dao = dao::profile_yaml_dao::new(cli.profile_path).await?;
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
    }
    Ok(())
}
