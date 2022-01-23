pub mod handlers;
pub mod templates;

use crate::handlers::{run_build, run_new};
use clap::{Parser, Subcommand};
use std::error::Error;

#[derive(Parser)]
#[clap(version, about)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    New {
        /// directory name
        name: String,
    },
    Build {
        input_dir: String,
        output_dir: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    match args.command {
        Command::New { name } => run_new(name),
        Command::Build {
            input_dir,
            output_dir,
        } => run_build(input_dir, output_dir),
    }
}
