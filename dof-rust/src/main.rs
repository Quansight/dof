use clap::{Parser, Subcommand, Args};

/// Simple program to greet a person
#[derive(Debug, Parser)]
#[command(name = "dof")]
#[command(version = "0.0")]
#[command(about = "checkpoint conda environments", long_about = None)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
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

    #[command(about = "Manage checkpoints", long_about = None)]
    Checkpoint(CheckpointArgs),
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct CheckpointArgs {
    #[command(subcommand)]
    command: Option<CheckpointCommands>,
}

#[derive(Debug, Subcommand)]
enum CheckpointCommands {
    #[command(about = "Delete a previous revision of the environment", long_about = None)]
    Delete { 
        #[arg(short, long, help="uuid of the revision to delete")]
        rev: Option<String> 
    },
    
    #[command(about = "Generate a diff of the current environment to the specified revision", long_about = None)]
    Diff { 
        #[arg(short, long, help="uuid of the revision to diff against")]
        rev: Option<String> 
    },
    
    #[command(about = "List all checkpoints for the current environment", long_about = None)]
    List {},
    
    #[command(about = "Install a previous revision of the environment", long_about = None)]
    Install {},
    
    #[command(about = "Create a lockfile for the current env and set a checkpoint", long_about = None)]
    Save { 
        #[arg(short, long, help="tags for the checkpoint")]
        tag: Option<String> 
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
        Some(Commands::Checkpoint(checkpoint)) => {
            println!("'checkpoint' was used")
        }
        None => {
            println!("idk");
        }
    }
}