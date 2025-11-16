mod state;
mod data;
mod ui;
mod app;
mod zellij;
mod tmux;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use state::tracker::{StateFile, SessionInfo};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cc-orchestra")]
#[command(about = "Real-time TUI dashboard for Claude Code sessions", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Track a Claude Code session (called by hooks)
    TrackSession {
        #[arg(long)]
        pid: u32,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        tmux_pane: Option<String>,
        #[arg(long)]
        tmux_session: Option<String>,
        #[arg(long)]
        tmux_window: Option<u32>,
    },
    /// Untrack a Claude Code session (called by hooks)
    UntrackSession {
        #[arg(long)]
        session_id: String,
    },
    /// Update last activity timestamp (called by Stop hook)
    UpdateActivity {
        #[arg(long)]
        session_id: String,
    },
    /// Update user input timestamp (called by UserPromptSubmit hook)
    UpdateUserInput {
        #[arg(long)]
        session_id: String,
    },
    /// Update asking question timestamp (called by PostToolUse[AskUserQuestion] hook)
    UpdateAskingQuestion {
        #[arg(long)]
        session_id: String,
    },
    /// Update awaiting permission timestamp (called by Notification[permission_prompt] hook)
    UpdateAwaitingPermission {
        #[arg(long)]
        session_id: String,
    },
}

fn get_state_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".claude/cc-orchestra-state.json"))
}

fn run_command(command: &Commands) -> Result<()> {
    match command {
        Commands::TrackSession { pid, session_id, tmux_pane, tmux_session, tmux_window } => {
            let mut state = StateFile::load(get_state_path()?)
                .context("Failed to load state file")?;
            let cwd = std::env::current_dir()
                .context("Failed to get current directory")?
                .to_string_lossy()
                .to_string();
            let tty = std::env::var("TTY").unwrap_or_else(|_| "unknown".to_string());

            let now = chrono::Utc::now().timestamp();
            state.add_session(session_id.clone(), SessionInfo {
                pid: *pid,
                tty,
                cwd,
                started_at: now,
                zellij_session: None, // TODO: detect Zellij
                zellij_tab: None,
                zellij_pane: None,
                tmux_pane: tmux_pane.clone(),
                tmux_session: tmux_session.clone(),
                tmux_window: *tmux_window,
                last_activity: now,  // Initialize to started_at
                user_input_at: 0,    // Initialize to 0 (no user input yet)
                asking_question_at: 0,  // Initialize to 0 (no question asked)
                awaiting_permission_at: 0,  // Initialize to 0 (no permission prompt)
            });
            state.save().context("Failed to save state file")?;
            println!("✓ Tracked session {session_id}");
            Ok(())
        }
        Commands::UntrackSession { session_id } => {
            let mut state = StateFile::load(get_state_path()?)
                .context("Failed to load state file")?;
            if state.remove_session(session_id).is_some() {
                state.save().context("Failed to save state file")?;
                println!("✓ Untracked session {session_id}");
            } else {
                println!("⚠ Session {session_id} not found");
            }
            Ok(())
        }
        Commands::UpdateActivity { session_id } => {
            let mut state = StateFile::load(get_state_path()?)
                .context("Failed to load state file")?;
            if let Some(session) = state.sessions.get_mut(session_id) {
                session.last_activity = chrono::Utc::now().timestamp();
                state.save().context("Failed to save state file")?;
                println!("✓ Updated activity for session {session_id}");
            } else {
                println!("⚠ Session {session_id} not found");
            }
            Ok(())
        }
        Commands::UpdateUserInput { session_id } => {
            let mut state = StateFile::load(get_state_path()?)
                .context("Failed to load state file")?;
            if let Some(session) = state.sessions.get_mut(session_id) {
                session.user_input_at = chrono::Utc::now().timestamp();
                state.save().context("Failed to save state file")?;
                println!("✓ Updated user input for session {session_id}");
            } else {
                println!("⚠ Session {session_id} not found");
            }
            Ok(())
        }
        Commands::UpdateAskingQuestion { session_id } => {
            let mut state = StateFile::load(get_state_path()?)
                .context("Failed to load state file")?;
            if let Some(session) = state.sessions.get_mut(session_id) {
                session.asking_question_at = chrono::Utc::now().timestamp();
                state.save().context("Failed to save state file")?;
                println!("✓ Updated asking question for session {session_id}");
            } else {
                println!("⚠ Session {session_id} not found");
            }
            Ok(())
        }
        Commands::UpdateAwaitingPermission { session_id } => {
            let mut state = StateFile::load(get_state_path()?)
                .context("Failed to load state file")?;
            if let Some(session) = state.sessions.get_mut(session_id) {
                session.awaiting_permission_at = chrono::Utc::now().timestamp();
                state.save().context("Failed to save state file")?;
                println!("✓ Updated awaiting permission for session {session_id}");
            } else {
                println!("⚠ Session {session_id} not found");
            }
            Ok(())
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(command) => {
            if let Err(e) = run_command(command) {
                eprintln!("Error: {e:?}");
                std::process::exit(1);
            }
        }
        None => {
            let app = app::App::new().unwrap();
            let mut dashboard = ui::dashboard::Dashboard::new(app);
            if let Err(e) = dashboard.run() {
                eprintln!("Error: {e:?}");
                std::process::exit(1);
            }
        }
    }
}
