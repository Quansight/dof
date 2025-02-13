use clap::{Parser, Subcommand};

/// Simple program to greet a person
#[derive(Parser)]
#[command(name = "dof")]
#[command(version = "0.0")]
#[command(about = "checkpoint conda environments", long_about = None)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Pull a checkpoint for a remote repo", long_about = None)]
    Pull {
        #[arg(short, long, help="namespace/environemnt:rev to pull from")]
        target: String,
    },
    #[command(about = "Push a checkpoint to a remote repo", long_about = None)]
    Push {
        #[arg(short, long, help="remote namespace/environemnt:rev to push to")]
        target: String,

        #[arg(short, long, help="local revision to push")]
        rev: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Pull { target }) => {
            println!("'pull' was used, target is: {:?}", target)
        }
        Some(Commands::Push { target, rev }) => {
            println!("'push' was used, target is: {:?}, rev is {:?}", target, rev)
        }
        None => {
            println!("idk");
        }
    }
}