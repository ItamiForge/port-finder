mod cli;
mod port;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use serde_json::json;
use std::process;

const EXIT_OK: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_CHECK_IN_USE: i32 = 2;
const EXIT_FIND_NOT_FOUND: i32 = 3;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(cmd) => run_command(cmd),
        None => tui::run().map(|_| EXIT_OK),
    };

    match result {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("Error: {err}");
            process::exit(EXIT_ERROR);
        }
    }
}

fn run_command(cmd: Command) -> Result<i32> {
    match cmd {
        Command::List { all, json } => {
            let ports = port::list_ports(all)?;
            if json {
                let entries = ports
                    .iter()
                    .map(port_info_json)
                    .collect::<Vec<serde_json::Value>>();
                println!("{}", serde_json::to_string_pretty(&entries)?);
            } else {
                port::print_ports(&ports);
            }
            Ok(EXIT_OK)
        }
        Command::Find { port, json } => {
            if let Some(info) = port::find_port(port)? {
                if json {
                    let payload = json!({
                        "port": port,
                        "in_use": true,
                        "entry": port_info_json(&info),
                    });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    port::print_ports(&[info]);
                }
                Ok(EXIT_OK)
            } else if json {
                let payload = json!({
                    "port": port,
                    "in_use": false,
                    "entry": serde_json::Value::Null,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(EXIT_FIND_NOT_FOUND)
            } else {
                println!("Port {} is not in use", port);
                Ok(EXIT_FIND_NOT_FOUND)
            }
        }
        Command::Kill { port, force } => {
            port::kill_port(port, force)?;
            Ok(EXIT_OK)
        }
        Command::Check { port, json } => {
            let available = port::is_available(port)?;
            if json {
                let payload = json!({
                    "port": port,
                    "available": available,
                    "in_use": !available,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if available {
                println!("Port {} is available", port);
            } else {
                println!("Port {} is in use", port);
            }
            if available {
                Ok(EXIT_OK)
            } else {
                Ok(EXIT_CHECK_IN_USE)
            }
        }
        Command::Scan { range } => {
            port::scan_range(&range)?;
            Ok(EXIT_OK)
        }
    }
}

fn port_info_json(info: &port::PortInfo) -> serde_json::Value {
    json!({
        "port": info.port,
        "protocol": info.protocol,
        "state": info.state,
        "pid": info.pid,
        "process_name": info.process_name,
        "local_addr": info.local_addr,
        "duration_secs": info.duration.as_secs(),
        "cpu_usage": info.cpu_usage,
        "memory": info.memory,
        "memory_human": info.format_memory(),
        "user": info.user,
        "command": info.command,
        "parent_pid": info.parent_pid,
    })
}
