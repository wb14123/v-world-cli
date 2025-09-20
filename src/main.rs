
use clap::{Parser, Subcommand};

mod model;

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

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::CreateProfile { path } => {
            model::profile::create_template_file(&path).unwrap();
            println!("Profile template file created successfully at {}", &path);
        }
    }
}
