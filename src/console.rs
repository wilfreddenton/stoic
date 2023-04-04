use color_eyre::{eyre::eyre, eyre::Report, Result};
use superconsole::{style::Stylize, Component, Line, SuperConsole};

#[derive(Debug)]
pub struct ConsoleState {
    pub report: Option<Report>,
    pub elapsed: Option<i64>,
    pub message: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug)]
pub struct Console;

impl Component for Console {
    fn draw_unchecked(
        &self,
        state: &superconsole::State,
        _dimensions: superconsole::Dimensions,
        _mode: superconsole::DrawMode,
    ) -> anyhow::Result<superconsole::Lines> {
        let mut lines = vec![];
        if let Ok(c_state) = state.get::<ConsoleState>() {
            if let Some(address) = &c_state.address {
                lines.push(Line(vec![format!("Serving @ {address}").try_into()?]));
            }

            if let Some(report) = &c_state.report {
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
            }

            if let Some(elapsed) = &c_state.elapsed {
                lines.push(Line(vec![
                    "".to_string().dark_green().try_into()?,
                    format!("Built in {elapsed} ms").try_into()?,
                ]));
            }

            if let Some(message) = &c_state.message {
                lines.push(Line(vec![message.to_string().try_into()?]));
            }
        } else {
            panic!("Could not parse console state");
        }
        Ok(lines)
    }
}

pub struct ConsoleHandle {
    console: SuperConsole,
    state: ConsoleState,
}

impl ConsoleHandle {
    pub fn new() -> Result<ConsoleHandle> {
        let console = SuperConsole::new(Box::new(Console {}))
            .ok_or(eyre!("Could not initialize superconsole"))?;
        Ok(ConsoleHandle {
            console,
            state: ConsoleState {
                report: None,
                elapsed: None,
                message: None,
                address: None,
            },
        })
    }

    fn render(&mut self) -> Result<()> {
        Ok(self
            .console
            .render(&superconsole::state!(&self.state))
            .map_err(|e| eyre!(Box::new(e)))?)
    }

    pub fn log_report(&mut self, report: Report) -> Result<()> {
        self.state.report = Some(report);
        self.render()?;
        self.state.report = None;
        Ok(())
    }

    pub fn log_elapsed(&mut self, elapsed: i64) -> Result<()> {
        self.state.elapsed = Some(elapsed);
        self.render()?;
        self.state.elapsed = None;
        Ok(())
    }

    pub fn log(&mut self, message: &str) -> Result<()> {
        self.state.message = Some(message.to_string());
        self.render()?;
        self.state.message = None;
        Ok(())
    }

    pub fn set_address(&mut self, address: &str) -> Result<()> {
        self.state.address = Some(address.to_string());
        Ok(self.render()?)
    }
}
