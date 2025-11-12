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

pub struct Dashboard {
    selected: usize,
}

impl Dashboard {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .constraints([Constraint::Min(0)])
                    .split(f.area());

                let items = vec![
                    ListItem::new("Session 1 - Working"),
                    ListItem::new("Session 2 - Waiting"),
                ];

                let list = List::new(items)
                    .block(Block::default()
                        .title("CC-ORCHESTRA")
                        .borders(Borders::ALL));

                f.render_widget(list, chunks[0]);
            })?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
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
