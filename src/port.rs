use anyhow::{anyhow, Result};
use colored::*;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
use std::io::{self, Write};
use std::net::TcpListener;
use sysinfo::{Pid, System};

#[derive(Debug, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub protocol: String,
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: String,
    pub local_addr: String,
    pub duration: std::time::Duration,
    pub cpu_usage: f32,
    pub memory: u64,
    pub user: String,
    pub command: String,
    pub parent_pid: Option<u32>,
}

impl PortInfo {
    pub fn format_duration(&self) -> String {
        let secs = self.duration.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            format!("{}h", secs / 3600)
        }
    }

    pub fn format_memory(&self) -> String {
        let mb = self.memory as f64 / 1024.0 / 1024.0;
        if mb < 1024.0 {
            format!("{:.1} MB", mb)
        } else {
            format!("{:.1} GB", mb / 1024.0)
        }
    }

    pub fn smart_label(&self) -> String {
        // Known ports
        match self.port {
            22 => return "SSH".to_string(),
            80 | 8080 => return "HTTP".to_string(),
            443 | 8443 => return "HTTPS".to_string(),
            3000..=3010 => return "Node/React".to_string(),
            5432 => return "Postgres".to_string(),
            3306 => return "MySQL".to_string(),
            6379 => return "Redis".to_string(),
            27017 => return "MongoDB".to_string(),
            _ => {}
        }

        // Known process names
        let name = self.process_name.to_lowercase();
        if name.contains("node") {
            return "Node".to_string();
        }
        if name.contains("python") {
            return "Python".to_string();
        }
        if name.contains("docker") {
            return "Docker".to_string();
        }
        if name.contains("chrome") {
            return "Chrome".to_string();
        }
        if name.contains("code") {
            return "VS Code".to_string();
        }

        self.process_name.clone()
    }
}

pub fn list_ports(all: bool) -> Result<Vec<PortInfo>> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let sockets = get_sockets_info(af_flags, proto_flags)?;
    let mut sys = System::new_all();
    sys.refresh_all();
    let users = sysinfo::Users::new_with_refreshed_list();

    let mut ports: Vec<PortInfo> = sockets
        .into_iter()
        .filter_map(|si| {
            let (port, protocol, state, local) = match &si.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => {
                    let state = format!("{:?}", tcp.state);
                    if !all && state != "Listen" {
                        return None;
                    }
                    (
                        tcp.local_port,
                        "TCP".to_string(),
                        state,
                        format!("{}:{}", tcp.local_addr, tcp.local_port),
                    )
                }
                ProtocolSocketInfo::Udp(udp) => (
                    udp.local_port,
                    "UDP".to_string(),
                    "-".to_string(),
                    format!("{}:{}", udp.local_addr, udp.local_port),
                ),
            };

            let pid = si.associated_pids.first().copied();
            let mut process_name = "-".to_string();
            let mut duration = std::time::Duration::from_secs(0);
            let mut cpu_usage = 0.0;
            let mut memory = 0;
            let mut user = "-".to_string();
            let mut command = "-".to_string();
            let mut parent_pid = None;

            if let Some(p) = pid {
                if let Some(proc) = sys.process(Pid::from_u32(p)) {
                    process_name = proc.name().to_string_lossy().to_string();
                    let start_time = proc.start_time();
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    if current_time > start_time {
                        duration = std::time::Duration::from_secs(current_time - start_time);
                    }

                    cpu_usage = proc.cpu_usage();
                    memory = proc.memory();

                    if let Some(uid) = proc.user_id() {
                        if let Some(u) = users.get_user_by_id(uid) {
                            user = u.name().to_string();
                        }
                    }

                    command = if !proc.cmd().is_empty() {
                        proc.cmd()
                            .iter()
                            .map(|s| s.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(" ")
                    } else {
                        process_name.clone()
                    };

                    parent_pid = proc.parent().map(|p| p.as_u32());
                }
            }

            Some(PortInfo {
                port,
                protocol,
                state,
                pid,
                process_name,
                local_addr: local,
                duration,
                cpu_usage,
                memory,
                user,
                command,
                parent_pid,
            })
        })
        .collect();

    ports.sort_by_key(|p| p.port);
    ports.dedup_by_key(|p| (p.port, p.protocol.clone()));

    Ok(ports)
}

pub fn find_port(port: u16) -> Result<Option<PortInfo>> {
    let ports = list_ports(true)?;
    Ok(ports.into_iter().find(|p| p.port == port))
}

pub fn print_ports(ports: &[PortInfo]) {
    if ports.is_empty() {
        println!("{}", "No ports found".yellow());
        return;
    }

    println!(
        "{:>6} {:>5} {:>12} {:>8} {:20} {}",
        "PORT".bold(),
        "PROTO".bold(),
        "STATE".bold(),
        "PID".bold(),
        "PROCESS".bold(),
        "LOCAL".bold()
    );
    println!("{}", "-".repeat(70));

    for p in ports {
        let state_colored = match p.state.as_str() {
            "Listen" => p.state.green(),
            "Established" => p.state.cyan(),
            "TimeWait" => p.state.yellow(),
            "CloseWait" => p.state.red(),
            _ => p.state.normal(),
        };

        let pid_str = p
            .pid
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:>6} {:>5} {:>12} {:>8} {:20} {}",
            p.port.to_string().cyan(),
            p.protocol,
            state_colored,
            pid_str,
            p.process_name.truncate_ellipsis(20),
            p.local_addr.dimmed()
        );
    }
}

pub fn kill_port(port: u16, force: bool) -> Result<()> {
    let info = find_port(port)?.ok_or_else(|| anyhow!("No process found on port {}", port))?;

    let pid = info
        .pid
        .ok_or_else(|| anyhow!("Cannot determine PID for port {}", port))?;

    if !force {
        print!(
            "Kill {} (PID {}) on port {}? [y/N] ",
            info.process_name.yellow(),
            pid,
            port
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted");
            return Ok(());
        }
    }

    let mut sys = System::new_all();
    sys.refresh_all();

    if let Some(process) = sys.process(Pid::from_u32(pid)) {
        if process.kill() {
            println!("{} Killed {} (PID {})", "✓".green(), info.process_name, pid);
        } else {
            return Err(anyhow!(
                "Failed to kill PID {} ({}). Try elevated privileges.",
                pid,
                info.process_name
            ));
        }
    } else {
        return Err(anyhow!("Process {} not found", pid));
    }

    Ok(())
}

pub fn is_available(port: u16) -> Result<bool> {
    match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

pub fn scan_range(range: &str) -> Result<()> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid range format. Use: 3000-4000"));
    }

    let start: u16 = parts[0].parse()?;
    let end: u16 = parts[1].parse()?;

    println!("Scanning ports {}-{}...\n", start, end);

    let mut available = Vec::new();
    let mut in_use = Vec::new();

    for port in start..=end {
        if is_available(port)? {
            available.push(port);
        } else {
            in_use.push(port);
        }
    }

    println!("{} {} ports available", "✓".green(), available.len());
    println!("{} {} ports in use", "✗".red(), in_use.len());

    if !in_use.is_empty() && in_use.len() <= 20 {
        println!(
            "\nIn use: {}",
            in_use
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(())
}

trait TruncateEllipsis {
    fn truncate_ellipsis(&self, max: usize) -> String;
}

impl TruncateEllipsis for String {
    fn truncate_ellipsis(&self, max: usize) -> String {
        if self.len() <= max {
            self.clone()
        } else {
            format!("{}…", &self[..max - 1])
        }
    }
}
