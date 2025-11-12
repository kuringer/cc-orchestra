mod state;

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

fn get_state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap();
    PathBuf::from(home).join(".claude/cc-orchestra-state.json")
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::TrackSession { pid, session_id }) => {
            let mut state = StateFile::load(get_state_path()).unwrap();
            let cwd = std::env::current_dir().unwrap().to_string_lossy().to_string();
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
            state.save().unwrap();
            println!("✓ Tracked session {}", session_id);
        }
        Some(Commands::UntrackSession { session_id }) => {
            let mut state = StateFile::load(get_state_path()).unwrap();
            if state.remove_session(session_id).is_some() {
                state.save().unwrap();
                println!("✓ Untracked session {}", session_id);
            } else {
                println!("⚠ Session {} not found", session_id);
            }
        }
        None => {
            println!("Starting dashboard...");
        }
    }
}
