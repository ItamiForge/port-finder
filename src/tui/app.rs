use crate::port::{self, PortInfo};
use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};
use ratatui::widgets::TableState;
use std::cmp::Ordering;
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
    pub group_mode: bool,
    pub inspect_mode: bool,
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
            group_mode: false,
            inspect_mode: false,
            clipboard,
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let mut ports = port::list_ports(self.show_all)?;
        self.sort_ports(&mut ports);
        self.ports = ports;
        self.visible_indices = (0..self.ports.len()).collect();

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

    pub fn kill_selected(&mut self) -> Result<()> {
        let Some(info) = self.selected_port().cloned() else {
            return Ok(());
        };

        if let Some(pid) = info.pid {
            let mut sys = System::new_all();
            sys.refresh_all();
            if let Some(process) = sys.process(Pid::from_u32(pid)) {
                process.kill();
                self.message = Some(format!("Killed PID {}", pid));
                self.refresh()?;
            }
        }
        Ok(())
    }

    pub fn selected_port(&self) -> Option<&PortInfo> {
        let selected = self.state.selected()?;
        let port_idx = *self.visible_indices.get(selected)?;
        self.ports.get(port_idx)
    }
}
