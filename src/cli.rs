use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pf")]
#[command(about = "Port finder - manage ports and processes")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// List all listening ports
    List {
        /// Show all connections, not just listening
        #[arg(short, long)]
        all: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Find process using a port
    Find {
        /// Port number
        port: u16,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Kill process on a port
    Kill {
        /// Port number
        port: u16,
        /// Force kill without confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Check if port is available
    Check {
        /// Port number
        port: u16,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Scan port range for availability
    Scan {
        /// Range (e.g., 3000-4000)
        range: String,
    },
}
