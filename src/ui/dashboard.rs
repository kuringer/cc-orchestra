use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};
use std::io;
use crate::app::App;
use crate::state::SessionState;

pub struct Dashboard {
    app: App,
}

impl Dashboard {
    pub fn new(app: App) -> Self {
        Self { app }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        loop {
            // Refresh data
            self.app.refresh()?;

            terminal.draw(|f| {
                let chunks = Layout::default()
                    .constraints([Constraint::Min(0)])
                    .split(f.area());

                let items: Vec<ListItem> = self.app.sessions()
                    .iter()
                    .enumerate()
                    .map(|(i, session)| {
                        let status_icon = match session.state {
                            SessionState::Working => "🟢",
                            SessionState::Waiting => "⏸️ ",
                            SessionState::Idle => "💤",
                            SessionState::Dead => "❌",
                        };

                        let text = format!("{} {} {} - {:?}",
                            if i == self.app.selected() { "►" } else { " " },
                            status_icon,
                            session.project_name,
                            session.state
                        );

                        ListItem::new(text)
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default()
                        .title("CC-ORCHESTRA")
                        .borders(Borders::ALL));

                f.render_widget(list, chunks[0]);
            })?;

            if event::poll(std::time::Duration::from_millis(2000))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => self.app.select_next(),
                        KeyCode::Up | KeyCode::Char('k') => self.app.select_previous(),
                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;

        Ok(())
    }
}
