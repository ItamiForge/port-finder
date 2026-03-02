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
                    let payload = find_json_payload(port, Some(&info));
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    port::print_ports(&[info]);
                }
                Ok(exit_code_for_find(true))
            } else if json {
                let payload = find_json_payload(port, None);
                println!("{}", serde_json::to_string_pretty(&payload)?);
                Ok(exit_code_for_find(false))
            } else {
                println!("Port {} is not in use", port);
                Ok(exit_code_for_find(false))
            }
        }
        Command::Kill { port, force } => {
            port::kill_port(port, force)?;
            Ok(EXIT_OK)
        }
        Command::Check { port, json } => {
            let available = port::is_available(port)?;
            if json {
                let payload = check_json_payload(port, available);
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if available {
                println!("Port {} is available", port);
            } else {
                println!("Port {} is in use", port);
            }
            Ok(exit_code_for_check(available))
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

fn find_json_payload(port: u16, info: Option<&port::PortInfo>) -> serde_json::Value {
    match info {
        Some(info) => json!({
            "port": port,
            "in_use": true,
            "entry": port_info_json(info),
        }),
        None => json!({
            "port": port,
            "in_use": false,
            "entry": serde_json::Value::Null,
        }),
    }
}

fn check_json_payload(port: u16, available: bool) -> serde_json::Value {
    json!({
        "port": port,
        "available": available,
        "in_use": !available,
    })
}

fn exit_code_for_find(found: bool) -> i32 {
    if found {
        EXIT_OK
    } else {
        EXIT_FIND_NOT_FOUND
    }
}

fn exit_code_for_check(available: bool) -> i32 {
    if available {
        EXIT_OK
    } else {
        EXIT_CHECK_IN_USE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::PortInfo;
    use std::time::Duration;

    #[test]
    fn exit_code_mappings_are_stable() {
        assert_eq!(exit_code_for_find(true), EXIT_OK);
        assert_eq!(exit_code_for_find(false), EXIT_FIND_NOT_FOUND);
        assert_eq!(exit_code_for_check(true), EXIT_OK);
        assert_eq!(exit_code_for_check(false), EXIT_CHECK_IN_USE);
    }

    #[test]
    fn find_json_payload_not_found_shape_is_stable() {
        let payload = find_json_payload(3000, None);
        assert_eq!(payload["port"], 3000);
        assert_eq!(payload["in_use"], false);
        assert!(payload["entry"].is_null());
    }

    #[test]
    fn check_json_payload_has_consistent_in_use_flag() {
        let available_payload = check_json_payload(8080, true);
        assert_eq!(available_payload["available"], true);
        assert_eq!(available_payload["in_use"], false);

        let in_use_payload = check_json_payload(8080, false);
        assert_eq!(in_use_payload["available"], false);
        assert_eq!(in_use_payload["in_use"], true);
    }

    #[test]
    fn port_info_json_contains_core_fields() {
        let info = PortInfo {
            port: 3000,
            protocol: "TCP".to_string(),
            state: "Listen".to_string(),
            pid: Some(1234),
            process_name: "node".to_string(),
            local_addr: "127.0.0.1:3000".to_string(),
            duration: Duration::from_secs(65),
            cpu_usage: 3.4,
            memory: 1024 * 1024,
            user: "dev".to_string(),
            command: "node server.js".to_string(),
            parent_pid: Some(1),
        };

        let payload = port_info_json(&info);
        assert_eq!(payload["port"], 3000);
        assert_eq!(payload["protocol"], "TCP");
        assert_eq!(payload["state"], "Listen");
        assert_eq!(payload["pid"], 1234);
        assert_eq!(payload["duration_secs"], 65);
        assert_eq!(payload["memory"], 1024 * 1024);
        assert_eq!(payload["memory_human"], "1.0 MB");
    }
}
