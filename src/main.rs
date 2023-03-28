use clap::Parser;
use std::{error::Error, path::Path};
use stoic::handlers::{run_build, run_new, run_watch};

#[derive(clap::Parser)]
#[clap(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    New {
        /// directory name
        name: String,
    },
    Build {
        input_dir: String,
        output_dir: String,
    },
    Watch {
        input_dir: String,
        output_dir: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    match args.command {
        Command::New { name } => run_new(Path::new(&name)).await,
        Command::Build {
            input_dir,
            output_dir,
        } => run_build(Path::new(&input_dir), Path::new(&output_dir), true).await,
        Command::Watch {
            input_dir,
            output_dir,
        } => run_watch(Path::new(&input_dir), Path::new(&output_dir)).await,
    }
}
