use crate::port::{self, PortInfo};
use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};
use ratatui::widgets::TableState;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use sysinfo::{Pid, System};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortColumn {
    Port,
    Pid,
    Memory,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolFilter {
    All,
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StateFilter {
    All,
    Listen,
    Established,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingKill {
    Single {
        pid: u32,
        port: u16,
        process_name: String,
    },
    Batch {
        pids: Vec<u32>,
    },
}

pub struct App {
    pub ports: Vec<PortInfo>,
    pub visible_indices: Vec<usize>,
    pub state: TableState,
    pub show_all: bool,
    pub filter_mode: bool,
    pub filter_text: String,
    pub message: Option<String>,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    pub protocol_filter: ProtocolFilter,
    pub state_filter: StateFilter,
    pub group_mode: bool,
    pub inspect_mode: bool,
    pub auto_refresh: bool,
    pub auto_refresh_interval_ms: u64,
    pub selected_ports: BTreeSet<usize>,
    pub pending_kill: Option<PendingKill>,
    clipboard: Option<ClipboardContext>,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut ports = port::list_ports(false)?;
        ports.sort_by_key(|p| p.port);

        let mut state = TableState::default();
        if !ports.is_empty() {
            state.select(Some(0));
        }

        let clipboard = ClipboardContext::new().ok();

        Ok(Self {
            visible_indices: (0..ports.len()).collect(),
            ports,
            state,
            show_all: false,
            filter_mode: false,
            filter_text: String::new(),
            message: None,
            sort_column: SortColumn::Port,
            sort_direction: SortDirection::Asc,
            protocol_filter: ProtocolFilter::All,
            state_filter: StateFilter::All,
            group_mode: false,
            inspect_mode: false,
            auto_refresh: false,
            auto_refresh_interval_ms: 1_000,
            selected_ports: BTreeSet::new(),
            pending_kill: None,
            clipboard,
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let mut ports = port::list_ports(self.show_all)?;
        self.sort_ports(&mut ports);
        self.ports = ports;
        self.visible_indices = (0..self.ports.len()).collect();
        self.selected_ports.clear();

        if self.visible_indices.is_empty() {
            self.state.select(None);
        } else if let Some(selected) = self.state.selected() {
            if selected >= self.visible_indices.len() {
                self.state.select(Some(self.visible_indices.len() - 1));
            }
        } else {
            self.state.select(Some(0));
        }
        Ok(())
    }

    fn sort_ports(&self, ports: &mut [PortInfo]) {
        match self.sort_column {
            SortColumn::Port => ports.sort_by(|a, b| a.port.cmp(&b.port)),
            SortColumn::Pid => ports.sort_by(|a, b| a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0))),
            SortColumn::Memory => ports.sort_by(|a, b| a.memory.cmp(&b.memory)),
            SortColumn::Cpu => ports.sort_by(|a, b| {
                a.cpu_usage
                    .partial_cmp(&b.cpu_usage)
                    .unwrap_or(Ordering::Equal)
            }),
        }

        if self.sort_direction == SortDirection::Desc {
            ports.reverse();
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_column = match self.sort_column {
            SortColumn::Port => SortColumn::Pid,
            SortColumn::Pid => SortColumn::Memory,
            SortColumn::Memory => SortColumn::Cpu,
            SortColumn::Cpu => SortColumn::Port,
        };
        let _ = self.refresh();
        self.message = Some(format!("Sorted by {:?}", self.sort_column));
    }

    pub fn toggle_sort_direction(&mut self) {
        self.sort_direction = match self.sort_direction {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        };
        let _ = self.refresh();
        self.message = Some(format!("Sort direction: {:?}", self.sort_direction));
    }

    pub fn cycle_protocol_filter(&mut self) {
        self.protocol_filter = match self.protocol_filter {
            ProtocolFilter::All => ProtocolFilter::Tcp,
            ProtocolFilter::Tcp => ProtocolFilter::Udp,
            ProtocolFilter::Udp => ProtocolFilter::All,
        };
        self.message = Some(format!("Protocol filter: {:?}", self.protocol_filter));
    }

    pub fn cycle_state_filter(&mut self) {
        self.state_filter = match self.state_filter {
            StateFilter::All => StateFilter::Listen,
            StateFilter::Listen => StateFilter::Established,
            StateFilter::Established => StateFilter::All,
        };
        self.message = Some(format!("State filter: {:?}", self.state_filter));
    }

    pub fn matches_quick_filters(&self, port: &PortInfo) -> bool {
        let protocol_match = match self.protocol_filter {
            ProtocolFilter::All => true,
            ProtocolFilter::Tcp => port.protocol.eq_ignore_ascii_case("tcp"),
            ProtocolFilter::Udp => port.protocol.eq_ignore_ascii_case("udp"),
        };

        let state_match = match self.state_filter {
            StateFilter::All => true,
            StateFilter::Listen => port.state.eq_ignore_ascii_case("listen"),
            StateFilter::Established => port.state.eq_ignore_ascii_case("established"),
        };

        protocol_match && state_match
    }

    pub fn toggle_auto_refresh(&mut self) {
        self.auto_refresh = !self.auto_refresh;
        self.message = Some(format!(
            "Auto-refresh: {} ({:.1}s)",
            if self.auto_refresh { "ON" } else { "OFF" },
            self.auto_refresh_interval_ms as f64 / 1000.0
        ));
    }

    pub fn increase_refresh_interval(&mut self) {
        self.auto_refresh_interval_ms = (self.auto_refresh_interval_ms + 500).min(10_000);
        self.message = Some(format!(
            "Auto-refresh interval: {:.1}s",
            self.auto_refresh_interval_ms as f64 / 1000.0
        ));
    }

    pub fn decrease_refresh_interval(&mut self) {
        self.auto_refresh_interval_ms = self.auto_refresh_interval_ms.saturating_sub(500).max(500);
        self.message = Some(format!(
            "Auto-refresh interval: {:.1}s",
            self.auto_refresh_interval_ms as f64 / 1000.0
        ));
    }

    pub fn begin_filter(&mut self) {
        self.filter_mode = true;
        self.message = Some("Filter mode: type text, Enter to apply, Esc to cancel".to_string());
    }

    pub fn push_filter_char(&mut self, value: char) {
        self.filter_text.push(value);
    }

    pub fn pop_filter_char(&mut self) {
        self.filter_text.pop();
    }

    pub fn apply_filter(&mut self) {
        self.filter_mode = false;
        self.message = Some(format!("Filter applied: '{}'", self.filter_text));
    }

    pub fn cancel_filter(&mut self) {
        self.filter_mode = false;
        self.filter_text.clear();
        self.message = Some("Filter cleared".to_string());
    }

    pub fn toggle_group(&mut self) {
        self.group_mode = !self.group_mode;
        self.state.select(Some(0));
        self.message = Some(format!(
            "Group Mode: {}",
            if self.group_mode { "ON" } else { "OFF" }
        ));
    }

    pub fn toggle_inspect(&mut self) {
        self.inspect_mode = !self.inspect_mode;
    }

    pub fn copy_selected(&mut self) {
        if let Some(info) = self.selected_port() {
            let text = info.local_addr.to_string();
            if let Some(ctx) = &mut self.clipboard {
                if ctx.set_contents(text.clone()).is_ok() {
                    self.message = Some(format!("Copied {}", text));
                } else {
                    self.message = Some("Clipboard error".to_string());
                }
            } else {
                self.message = Some("Clipboard unavailable".to_string());
            }
        }
    }

    pub fn next(&mut self) {
        if self.inspect_mode {
            return;
        }
        if self.visible_indices.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.visible_indices.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn prev(&mut self) {
        if self.inspect_mode {
            return;
        }
        if self.visible_indices.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.visible_indices.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn first(&mut self) {
        if !self.visible_indices.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn last(&mut self) {
        if !self.visible_indices.is_empty() {
            self.state.select(Some(self.visible_indices.len() - 1));
        }
    }

    pub fn page_down(&mut self, step: usize) {
        if self.visible_indices.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let target = (current + step).min(self.visible_indices.len() - 1);
        self.state.select(Some(target));
    }

    pub fn page_up(&mut self, step: usize) {
        if self.visible_indices.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let target = current.saturating_sub(step);
        self.state.select(Some(target));
    }

    pub fn toggle_all(&mut self) {
        self.show_all = !self.show_all;
        let _ = self.refresh();
    }

    pub fn toggle_select_selected(&mut self) {
        let Some(index) = self.selected_port_index() else {
            return;
        };

        if self.selected_ports.contains(&index) {
            self.selected_ports.remove(&index);
        } else {
            self.selected_ports.insert(index);
        }

        self.message = Some(format!("Selected {} row(s)", self.selected_ports.len()));
    }

    pub fn has_pending_kill(&self) -> bool {
        self.pending_kill.is_some()
    }

    pub fn pending_kill_prompt(&self) -> Option<String> {
        match &self.pending_kill {
            Some(PendingKill::Single {
                pid,
                port,
                process_name,
            }) => Some(format!(
                "Kill PID {} ({}) on port {}? [y/Enter=yes, n/Esc=no]",
                pid, process_name, port
            )),
            Some(PendingKill::Batch { pids }) => Some(format!(
                "Kill {} selected process(es)? [y/Enter=yes, n/Esc=no]",
                pids.len()
            )),
            None => None,
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_ports.clear();
        self.message = Some("Selection cleared".to_string());
    }

    pub fn request_kill_selected(&mut self) {
        let Some(info) = self.selected_port().cloned() else {
            self.message = Some("No selected row".to_string());
            return;
        };

        let Some(pid) = info.pid else {
            self.message = Some("Selected row has no killable PID".to_string());
            return;
        };

        self.pending_kill = Some(PendingKill::Single {
            pid,
            port: info.port,
            process_name: info.process_name,
        });
        self.message = Some("Kill pending confirmation".to_string());
    }

    pub fn request_kill_selected_batch(&mut self) {
        if self.selected_ports.is_empty() {
            self.message = Some("No selected rows".to_string());
            return;
        }

        let mut pids_to_kill = BTreeSet::new();
        for index in &self.selected_ports {
            if let Some(info) = self.ports.get(*index) {
                if let Some(pid) = info.pid {
                    pids_to_kill.insert(pid);
                }
            }
        }

        if pids_to_kill.is_empty() {
            self.message = Some("No killable PID in selected rows".to_string());
            return;
        }

        self.pending_kill = Some(PendingKill::Batch {
            pids: pids_to_kill.into_iter().collect(),
        });
        self.message = Some("Batch kill pending confirmation".to_string());
    }

    pub fn confirm_pending_kill(&mut self) -> Result<()> {
        let Some(pending_kill) = self.pending_kill.take() else {
            return Ok(());
        };

        match pending_kill {
            PendingKill::Single {
                pid,
                port: _,
                process_name: _,
            } => {
                let killed = self.kill_pids([pid]);
                self.message = if killed == 1 {
                    Some(format!("Killed PID {}", pid))
                } else {
                    Some(format!("PID {} was not running", pid))
                };
            }
            PendingKill::Batch { pids } => {
                let requested = pids.len();
                let killed = self.kill_pids(pids);
                self.message = Some(format!(
                    "Batch kill complete: {}/{} process(es)",
                    killed, requested
                ));
            }
        }

        self.refresh()?;
        Ok(())
    }

    pub fn cancel_pending_kill(&mut self) {
        if self.pending_kill.take().is_some() {
            self.message = Some("Kill canceled".to_string());
        }
    }

    fn kill_pids(&self, pids: impl IntoIterator<Item = u32>) -> usize {
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut killed = 0usize;
        for pid in pids {
            if let Some(process) = sys.process(Pid::from_u32(pid)) {
                process.kill();
                killed += 1;
            }
        }
        killed
    }

    pub fn selected_port_index(&self) -> Option<usize> {
        let selected = self.state.selected()?;
        self.visible_indices.get(selected).copied()
    }

    pub fn selected_port(&self) -> Option<&PortInfo> {
        let port_idx = self.selected_port_index()?;
        self.ports.get(port_idx)
    }
}
