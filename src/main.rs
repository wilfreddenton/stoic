use clap::Parser;
use color_eyre::eyre::eyre;
use color_eyre::eyre::Result;
use color_eyre::Report;
use std::path::Path;
use std::time::Duration;
use stoic::handlers::{run_build, run_new, run_watch};
use superconsole::style::Stylize;
use superconsole::Component;
use superconsole::Line;
use superconsole::{Span, SuperConsole};

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

#[derive(Debug)]
struct HelloWorld;

impl Component for HelloWorld {
    fn draw_unchecked(
        &self,
        state: &superconsole::State,
        _dimensions: superconsole::Dimensions,
        _mode: superconsole::DrawMode,
    ) -> anyhow::Result<superconsole::Lines> {
        // let width = 30;
        // let (iteration, total) = state.get::<(usize, usize)>().unwrap_or(&(0, 1));
        // let percentage = *iteration as f64 / *total as f64;
        // let amount = (percentage * width as f64).ceil() as usize;
        // let loading_bar = format!(
        //     "[{test:=>bar_amt$}{test2:padding_amt$}] {}/{}: building...",
        //     iteration,
        //     total,
        //     test = ">",
        //     test2 = "",
        //     bar_amt = amount,
        //     padding_amt = width - amount,
        // );
        // lines.push(Line(vec![loading_bar.try_into()?]));
        let mut lines = vec![];
        if let Ok(report) = state.get::<Report>() {
            lines.push(Line(vec![
                "".to_string().dark_red().try_into()?,
                "Build failed".try_into()?,
            ]));
            lines.push(Line(vec!["Error:".try_into()?]));
            for (i, e) in report.chain().enumerate() {
                lines.push(Line(vec![
                    format!("    {i}: ").try_into()?,
                    e.to_string().dark_red().try_into()?,
                ]));
            }
        } else if let Ok(elapsed) = state.get::<i64>() {
            lines.push(Line(vec![
                "".to_string().dark_green().try_into()?,
                format!("Built in {elapsed} ms").try_into()?,
            ]));
        } else {
            lines.push(Line(vec!["Building...".try_into()?]));
        }
        Ok(lines)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;
    let hello_world = HelloWorld {};
    let mut console = SuperConsole::new(Box::new(hello_world)).unwrap();
    let args = Args::parse();
    if let Err(report) = match args.command {
        Command::New { name } => run_new(Path::new(&name)).await,
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
        console.render(&superconsole::state!(&report)).unwrap();
    }
    Ok(())
}
