pub mod assets;
pub mod handlers;
pub mod templates;
pub mod utils;

use crate::handlers::{run_build, run_new};
use clap::Parser;
use std::error::Error;

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

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    match args.command {
        Command::New { name } => run_new(name),
        Command::Build {
            input_dir,
            output_dir,
        } => run_build(&input_dir, &output_dir, true),
        Command::Watch {input_dir, output_dir} => Ok(()),
    }
}
