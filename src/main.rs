
use clap::{Parser, Subcommand};

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
        path: String,
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::CreateProfile { path } => {
            model::profile::create_template_file(&path).await.unwrap();
            println!("Profile template file created successfully at {}", &path);
        }
    }
}
