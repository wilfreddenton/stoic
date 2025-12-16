use color_eyre::{eyre::eyre, eyre::Report, Result};
use superconsole::{
    style::Color, Component, Dimensions, DrawMode, Line, Lines, Span, SuperConsole,
};

#[derive(Debug, Default)]
pub struct ConsoleState {
    pub report: Option<Report>,
    pub elapsed: Option<i64>,
    pub message: Option<String>,
    pub address: Option<String>,
}

/// A temporary view component that wraps the state for rendering.
struct ConsoleView<'a> {
    state: &'a ConsoleState,
}

impl<'a> Component for ConsoleView<'a> {
    // The method is still 'draw_unchecked', not 'draw'
    fn draw_unchecked(&self, _dimensions: Dimensions, _mode: DrawMode) -> anyhow::Result<Lines> {
        let mut lines = Lines::new();

        if let Some(address) = &self.state.address {
            lines.push(Line::from_iter(vec![Span::new_unstyled(format!(
                "Serving @ {address}"
            ))?]));
        }

        if let Some(report) = &self.state.report {
            // Span::new_colored is the cleanest way to add color in v0.2+
            // to avoid signature confusion with new_styled.
            lines.push(Line::from_iter(vec![Span::new_colored(
                "Build failed",
                Color::DarkRed,
            )?]));

            lines.push(Line::from_iter(vec![Span::new_unstyled("Error:")?]));

            for (i, e) in report.chain().enumerate() {
                lines.push(Line::from_iter(vec![
                    Span::new_unstyled(format!("    {i}: "))?,
                    Span::new_colored(&e.to_string(), Color::DarkRed)?,
                ]));
            }
        }

        if let Some(elapsed) = &self.state.elapsed {
            lines.push(Line::from_iter(vec![Span::new_colored(
                &format!("Built in {elapsed} ms"),
                Color::DarkGreen,
            )?]));
        }

        if let Some(message) = &self.state.message {
            lines.push(Line::from_iter(vec![Span::new_unstyled(message)?]));
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
        let console = SuperConsole::new().ok_or(eyre!("Could not initialize superconsole"))?;

        Ok(ConsoleHandle {
            console,
            state: ConsoleState::default(),
        })
    }

    fn render(&mut self) -> Result<()> {
        let view = ConsoleView { state: &self.state };
        self.console.render(&view).map_err(|e| eyre!(e))
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
        self.render()?;
        Ok(())
    }
}
