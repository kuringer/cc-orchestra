use clap::{Parser, Subcommand};

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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::TrackSession { pid, session_id }) => {
            println!("Tracking session {} with PID {}", session_id, pid);
        }
        Some(Commands::UntrackSession { session_id }) => {
            println!("Untracking session {}", session_id);
        }
        None => {
            println!("Starting dashboard...");
        }
    }
}
