
use clap::{Parser, Subcommand};
use crate::dao::profile_dao::ProfileDao;
use crate::model::profile::Profile;

mod model;
mod dao;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    CreateProfile {
        id: String,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let profile_dao = dao::profile_yaml_dao::new("./profiles".to_string()).await?;
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
