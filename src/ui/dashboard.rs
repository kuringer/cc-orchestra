use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
                // Create main layout with header, body, and footer
                let chunks = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),   // Header
                        Constraint::Min(0),      // Session list
                        Constraint::Length(3),   // Footer
                    ])
                    .split(f.area());

                // Header with session count and timestamp
                let header = Paragraph::new(format!(
                    "CC-ORCHESTRA          {} Active Sessions          Updated: {}",
                    self.app.sessions().len(),
                    chrono::Local::now().format("%H:%M:%S")
                ))
                .block(Block::default().borders(Borders::ALL));
                f.render_widget(header, chunks[0]);

                // Session list with detailed formatting
                let items: Vec<ListItem> = self.app.sessions()
                    .iter()
                    .enumerate()
                    .map(|(i, session)| {
                        let status_icon = match session.state {
                            SessionState::Working => "🟢 Working",
                            SessionState::Waiting => "⏸️  Waiting",
                            SessionState::Idle => "💤 Idle",
                            SessionState::Dead => "❌ Dead",
                        };

                        // Calculate age from last message
                        let age = if let Some(ref msg) = session.last_message {
                            let now = chrono::Utc::now().timestamp() * 1000;
                            let secs = (now - msg.timestamp) / 1000;
                            if secs < 60 {
                                format!("{}s ago", secs)
                            } else if secs < 3600 {
                                format!("{}m ago", secs / 60)
                            } else {
                                format!("{}h ago", secs / 3600)
                            }
                        } else {
                            "unknown".to_string()
                        };

                        let text = format!(
                            "{} {:20} {:15} PID:{:6} {}",
                            if i == self.app.selected() { "►" } else { " " },
                            session.project_name,
                            status_icon,
                            session.info.pid,
                            age
                        );

                        ListItem::new(text)
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(list, chunks[1]);

                // Footer with keybindings
                let footer = Paragraph::new("[↑↓/jk] Navigate  [Enter] Jump  [r] Refresh  [q] Quit")
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(footer, chunks[2]);
            })?;

            if event::poll(std::time::Duration::from_millis(2000))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => self.app.select_next(),
                        KeyCode::Up | KeyCode::Char('k') => self.app.select_previous(),
                        KeyCode::Char('r') => { /* Force refresh - happens at top of loop */ }
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
