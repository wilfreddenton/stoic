use clap::Parser;
use color_eyre::eyre::Result;
use std::path::Path;
use stoic::handlers::{run_build, run_new, run_watch};
use stoic::console::ConsoleHandle;

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
async fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;
    let mut console = ConsoleHandle::new()?;
    let args = Args::parse();
    if let Err(report) = match args.command {
        Command::New { name } => run_new(&mut console, Path::new(&name)).await,
        Command::Build {
            input_dir,
            output_dir,
        } => {
            run_build(
                &mut console,
                Path::new(&input_dir),
                Path::new(&output_dir),
                true,
            )
            .await
        }
        Command::Watch {
            input_dir,
            output_dir,
        } => run_watch(&mut console, Path::new(&input_dir), Path::new(&output_dir)).await,
    } {
        console.log_report(report)?;
    }
    Ok(())
}
