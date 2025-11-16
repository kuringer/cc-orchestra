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
use std::io::Write;
use crate::app::App;
use crate::state::SessionState;

pub struct Dashboard {
    app: App,
    log_file: std::fs::File,
}

impl Dashboard {
    pub fn new(app: App) -> Self {
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/cc-orchestra-debug.log")
            .expect("Failed to open log file");
        Self { app, log_file }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        loop {
            // Refresh data
            self.app.refresh()?;

            let draw_start = std::time::Instant::now();
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
                            SessionState::Active => "🟢 Active",
                            SessionState::Idle => "💤 Idle",
                            SessionState::Dead => "❌ Dead",
                        };

                        // Calculate last activity age
                        let last_active = {
                            let now = chrono::Utc::now().timestamp();
                            // Fallback: if last_activity is 0 (old sessions), use started_at
                            let last_activity = if session.info.last_activity == 0 {
                                session.info.started_at
                            } else {
                                session.info.last_activity
                            };
                            let secs = now - last_activity;
                            if secs < 5 {
                                "now".to_string()
                            } else if secs < 60 {
                                format!("{}s ago", secs)
                            } else if secs < 3600 {
                                format!("{}m ago", secs / 60)
                            } else {
                                format!("{}h ago", secs / 3600)
                            }
                        };

                        // Check if session is in tmux
                        let tmux_indicator = if session.info.tmux_pane.is_none() {
                            "⚠️ "
                        } else {
                            ""
                        };

                        let text = format!(
                            "{} {}{:20} {:15} PID:{:6} {}",
                            if i == self.app.selected() { "►" } else { " " },
                            tmux_indicator,
                            session.project_name,
                            status_icon,
                            session.info.pid,
                            last_active
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

            let draw_time = draw_start.elapsed();
            if draw_time.as_millis() > 100 {
                let _ = writeln!(self.log_file, "[SLOW DRAW] terminal.draw() took {:?}", draw_time);
            }

            if event::poll(std::time::Duration::from_millis(10))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => self.app.select_next(),
                        KeyCode::Up | KeyCode::Char('k') => self.app.select_previous(),
                        KeyCode::Char('r') => { /* Force refresh - happens at top of loop */ }
                        KeyCode::Enter => {
                            // Focus selected session's tmux pane (works from outside tmux)
                            if let Some(session) = self.app.sessions().get(self.app.selected()) {
                                if let Some(ref pane_id) = session.info.tmux_pane {
                                    // Focus the tmux pane (affects other window with tmux)
                                    let _ = crate::tmux::client::focus_pane(pane_id);
                                    // Don't exit dashboard - let it stay open
                                }
                            }
                        }
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
