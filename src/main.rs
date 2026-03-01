mod cli;
mod port;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => run_command(cmd),
        None => tui::run(),
    }
}

fn run_command(cmd: Command) -> Result<()> {
    match cmd {
        Command::List { all } => {
            let ports = port::list_ports(all)?;
            port::print_ports(&ports);
        }
        Command::Find { port } => {
            if let Some(info) = port::find_port(port)? {
                port::print_ports(&[info]);
            } else {
                println!("Port {} is not in use", port);
            }
        }
        Command::Kill { port, force } => {
            port::kill_port(port, force)?;
        }
        Command::Check { port } => {
            if port::is_available(port)? {
                println!("Port {} is available", port);
            } else {
                println!("Port {} is in use", port);
            }
        }
        Command::Scan { range } => {
            port::scan_range(&range)?;
        }
    }
    Ok(())
}
