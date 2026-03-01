use super::app::{App, SortColumn};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};
use std::cmp::Ordering;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    if app.inspect_mode {
        render_inspect_popup(frame, app);
        return;
    }

    let filtered_ports: Vec<(usize, &crate::port::PortInfo)> = if app.filter_text.is_empty() {
        app.ports
            .iter()
            .enumerate()
            .filter(|(_, port)| app.matches_quick_filters(port))
            .collect()
    } else {
        app.ports
            .iter()
            .enumerate()
            .filter(|p| {
                app.matches_quick_filters(p.1)
                    && (app.filter_text.is_empty()
                        || p.1.port.to_string().contains(&app.filter_text)
                        || p.1
                            .process_name
                            .to_lowercase()
                            .contains(&app.filter_text.to_lowercase()))
            })
            .collect()
    };

    let mut visible_indices = Vec::new();

    let rows: Vec<Row> = if app.group_mode {
        use std::collections::HashMap;
        let mut groups: HashMap<u32, Vec<(usize, &crate::port::PortInfo)>> = HashMap::new();
        let mut no_pid: Vec<(usize, &crate::port::PortInfo)> = Vec::new();

        for p in filtered_ports {
            if let Some(pid) = p.1.pid {
                groups.entry(pid).or_default().push(p);
            } else {
                no_pid.push(p);
            }
        }

        let mut sorted_groups: Vec<_> = groups.into_iter().collect();
        sorted_groups.sort_by(|(_, a_ports), (_, b_ports)| {
            let a = a_ports[0].1;
            let b = b_ports[0].1;
            match app.sort_column {
                SortColumn::Port => a.port.cmp(&b.port),
                SortColumn::Pid => a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0)),
                SortColumn::Memory => b.memory.cmp(&a.memory),
                SortColumn::Cpu => b
                    .cpu_usage
                    .partial_cmp(&a.cpu_usage)
                    .unwrap_or(Ordering::Equal),
            }
        });

        let mut group_rows = Vec::new();

        for (_, process_ports) in &sorted_groups {
            let representative = process_ports[0];
            let p = representative.1;
            visible_indices.push(representative.0);
            let selected_marker = if app.selected_ports.contains(&representative.0) {
                "*"
            } else {
                " "
            };

            let ports_str = process_ports
                .iter()
                .map(|(_, port)| port.port.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let count = process_ports.len();
            let port_display = if count > 1 {
                format!("{} ({})", ports_str, count)
            } else {
                ports_str
            };

            let state_style = match p.state.as_str() {
                "Listen" => Style::default().fg(Color::Green),
                _ => Style::default(),
            };

            group_rows.push(Row::new(vec![
                Cell::from(selected_marker),
                Cell::from(port_display).style(Style::default().fg(Color::Cyan)),
                Cell::from(p.protocol.as_str()),
                Cell::from(p.state.as_str()).style(state_style),
                Cell::from(p.pid.map(|id| id.to_string()).unwrap_or("-".to_string())),
                Cell::from(p.smart_label()),
                Cell::from(p.local_addr.as_str()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(p.format_duration()).style(Style::default().fg(Color::Magenta)),
                Cell::from(format!("{:.1}%", p.cpu_usage)),
                Cell::from(p.format_memory()),
                Cell::from(p.user.as_str()),
            ]));
        }

        for (index, p) in no_pid {
            visible_indices.push(index);
            let selected_marker = if app.selected_ports.contains(&index) {
                "*"
            } else {
                " "
            };
            let state_style = match p.state.as_str() {
                "Listen" => Style::default().fg(Color::Green),
                _ => Style::default(),
            };
            group_rows.push(Row::new(vec![
                Cell::from(selected_marker),
                Cell::from(p.port.to_string()).style(Style::default().fg(Color::Cyan)),
                Cell::from(p.protocol.as_str()),
                Cell::from(p.state.as_str()).style(state_style),
                Cell::from("-"),
                Cell::from(p.smart_label()),
                Cell::from(p.local_addr.as_str()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(p.format_duration()).style(Style::default().fg(Color::Magenta)),
                Cell::from("-"),
                Cell::from("-"),
                Cell::from("-"),
            ]));
        }

        group_rows
    } else {
        filtered_ports
            .iter()
            .map(|(index, p)| {
                visible_indices.push(*index);
                let selected_marker = if app.selected_ports.contains(index) {
                    "*"
                } else {
                    " "
                };
                let state_style = match p.state.as_str() {
                    "Listen" => Style::default().fg(Color::Green),
                    "Established" => Style::default().fg(Color::Cyan),
                    "TimeWait" => Style::default().fg(Color::Yellow),
                    "CloseWait" => Style::default().fg(Color::Red),
                    _ => Style::default(),
                };

                Row::new(vec![
                    Cell::from(selected_marker),
                    Cell::from(p.port.to_string()).style(Style::default().fg(Color::Cyan)),
                    Cell::from(p.protocol.as_str()),
                    Cell::from(p.state.as_str()).style(state_style),
                    Cell::from(p.pid.map(|id| id.to_string()).unwrap_or("-".to_string())),
                    Cell::from(p.smart_label()),
                    Cell::from(p.local_addr.as_str()).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(p.format_duration()).style(Style::default().fg(Color::Magenta)),
                    Cell::from(format!("{:.1}%", p.cpu_usage)),
                    Cell::from(p.format_memory()),
                    Cell::from(p.user.as_str()),
                ])
            })
            .collect()
    };

    app.visible_indices = visible_indices;
    if app.visible_indices.is_empty() {
        app.state.select(None);
    } else {
        let clamped = app
            .state
            .selected()
            .unwrap_or(0)
            .min(app.visible_indices.len() - 1);
        app.state.select(Some(clamped));
    }

    let header_text = format!(
        " Port Finder  •  Visible: {}  •  Total: {}  •  Selected: {}  •  Mode: {}  •  Sort: {:?} ({:?})  •  Proto: {:?}  •  State: {:?}  •  Auto: {} ({:.1}s) ",
        rows.len(),
        app.ports.len(),
        app.selected_ports.len(),
        if app.show_all { "ALL" } else { "LISTEN" },
        app.sort_column,
        app.sort_direction,
        app.protocol_filter,
        app.state_filter,
        if app.auto_refresh { "ON" } else { "OFF" },
        app.auto_refresh_interval_ms as f64 / 1000.0
    );
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let header_cells = [
        "SEL", "PORT(S)", "PROTO", "STATE", "PID", "PROCESS", "LOCAL", "TIME", "CPU", "MEM", "USER",
    ];
    let header = Row::new(header_cells).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(
        Table::new(
            rows,
            [
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Length(6),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Length(20),
                Constraint::Min(20),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Ports "))
        .row_highlight_style(Style::default().bg(Color::DarkGray)),
        chunks[1],
        &mut app.state,
    );

    let mode_indicator = if app.show_all { "ALL" } else { "LISTEN" };
    let group_indicator = if app.group_mode { "GRP:ON" } else { "GRP:OFF" };
    let filter_indicator = if app.filter_mode {
        format!("FILTER: {}_", app.filter_text)
    } else if app.filter_text.is_empty() {
        "FILTER: (none)".to_string()
    } else {
        format!("FILTER: {}", app.filter_text)
    };
    let msg = app.message.as_deref().unwrap_or("");
    let kill_hint = if app.has_pending_kill() {
        " [y/Enter]confirm [n/Esc]cancel"
    } else {
        ""
    };
    let footer = Paragraph::new(format!(
        " [q]uit [r]efresh [t]auto [+/−]interval [a]ll({}) [g]roup({}) [s]ort [d]dir [p]proto [w]state [/]filter [z]reset-filters [j/k,↑/↓]move [Pg]page [Home/End] [Space]select [v]vis-toggle [u]pid-toggle [x]clear [K]kill [B]batch-kill [Enter]inspect [c]opy{}  {}  {}",
        mode_indicator, group_indicator, kill_hint, filter_indicator, msg
    ))
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, chunks[2]);

    if app.inspect_mode {
        render_inspect_popup(frame, app);
    }

    if app.has_pending_kill() {
        render_confirm_popup(frame, app);
    }
}

fn render_inspect_popup(frame: &mut Frame, app: &App) {
    if let Some(port) = app.selected_port() {
        let area = centered_rect(60, 60, frame.area());

        let text = vec![
            Line::from(vec![
                Span::raw("Port: "),
                Span::styled(port.port.to_string(), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::raw("PID: "),
                Span::styled(
                    port.pid.map(|p| p.to_string()).unwrap_or_default(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::raw("Parent PID: "),
                Span::styled(
                    port.parent_pid
                        .map(|p| p.to_string())
                        .unwrap_or("-".to_string()),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::raw("Process: "),
                Span::styled(
                    port.process_name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("Command: "),
                Span::styled(port.command.clone(), Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("User: "),
                Span::styled(port.user.clone(), Style::default().fg(Color::Blue)),
            ]),
            Line::from(vec![
                Span::raw("Memory: "),
                Span::styled(port.format_memory(), Style::default().fg(Color::Magenta)),
            ]),
            Line::from(vec![
                Span::raw("CPU: "),
                Span::styled(
                    format!("{:.1}%", port.cpu_usage),
                    Style::default().fg(Color::Red),
                ),
            ]),
            Line::from(vec![
                Span::raw("Local: "),
                Span::styled(
                    port.local_addr.clone(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::raw("Protocol: "),
                Span::styled(port.protocol.clone(), Style::default()),
            ]),
            Line::from(vec![
                Span::raw("State: "),
                Span::styled(port.state.clone(), Style::default()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press [Esc] or [Enter] to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Process Details "),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }
}

fn render_confirm_popup(frame: &mut Frame, app: &App) {
    let Some(prompt) = app.pending_kill_prompt() else {
        return;
    };

    let area = centered_rect(52, 24, frame.area());
    let text = vec![
        Line::from(Span::styled(
            "Confirm Kill",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(prompt),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" Confirm "))
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1]);

    layout[1]
}
