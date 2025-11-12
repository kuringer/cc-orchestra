mod state;
mod data;

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
    },
    /// Untrack a Claude Code session (called by hooks)
    UntrackSession {
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
        Commands::TrackSession { pid, session_id } => {
            let mut state = StateFile::load(get_state_path()?)
                .context("Failed to load state file")?;
            let cwd = std::env::current_dir()
                .context("Failed to get current directory")?
                .to_string_lossy()
                .to_string();
            let tty = std::env::var("TTY").unwrap_or_else(|_| "unknown".to_string());

            state.add_session(session_id.clone(), SessionInfo {
                pid: *pid,
                tty,
                cwd,
                started_at: chrono::Utc::now().timestamp(),
                zellij_session: None, // TODO: detect Zellij
                zellij_tab: None,
                zellij_pane: None,
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
            println!("Starting dashboard...");
        }
    }
}
