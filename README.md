# port-finder

Cross-platform CLI and TUI for finding and reclaiming busy ports.

## Features

- List active ports with process details
- Find what owns a specific port
- Kill process by port
- Check whether a port is available
- Scan ranges for available and in-use ports
- Interactive TUI mode (`pf`)
- Selection persistence across refresh and auto-refresh
- Clear kill feedback (killed, missing, permission/protected failures)
- Works on macOS, Linux, and Windows

## Install

### From GitHub

```bash
cargo install --git https://github.com/ItamiForge/port-finder.git
```

### From local source

```bash
cargo install --path .
```

### Verify

```bash
pf --version
```

## Quick usage

```bash
pf
pf list
pf find 3000
pf find 3000 --json
pf kill 3000
pf check 8080 --json
pf scan 3000-4000
```

## Command reference

- `pf`: launch interactive TUI
- `pf list [--all] [--json]`: list active ports (`--all` includes non-listen states)
- `pf find <port> [--json]`: show process details for one port
- `pf kill <port> [--force]`: terminate process on a port
- `pf check <port> [--json]`: return availability status
- `pf scan <start-end>`: scan a range like `3000-4000`

### JSON output

Use `--json` on `list`, `find`, and `check` for machine-readable output.

- `pf list --json`: JSON array of port entries
- `pf find 3000 --json`: object with `in_use` and `entry`
- `pf check 8080 --json`: object with `available` and `in_use`

### Exit codes

CLI commands use stable exit codes for automation:

- `0`: success (`list`, `scan`, `kill`, `find` found, `check` available)
- `1`: command/runtime error
- `2`: `check` found the port in use
- `3`: `find` did not find an active owner for the port

## TUI controls

- `q` / `Esc`: quit
- `r`: refresh
- `t`: toggle auto-refresh
- `+` / `-`: increase/decrease auto-refresh interval
- `↑` / `↓` or `j` / `k`: navigate
- `n` / `N`: jump to next/previous selected row
- `PgUp` / `PgDn`: page navigation
- `Home` / `End`: jump to first/last row
- `Enter`: inspect selected process
- `Space`: toggle row selection
- `v`: toggle selection for all visible rows
- `i`: invert selection for visible rows
- `u`: toggle selection for all rows with selected PID
- `x`: clear selected rows
- `K`: stage kill for selected process (requires confirmation)
- `B`: stage batch kill for selected rows (requires confirmation)
- `y` / `Enter`: confirm pending kill
- `n` / `Esc`: cancel pending kill
- `a`: toggle all/listening view
- `g`: toggle grouped view
- `s`: cycle sort column
- `d`: toggle sort direction
- `p`: cycle protocol filter (All/TCP/UDP)
- `w`: cycle state filter (All/Listen/Established)
- `/`: enter filter mode (type to filter, `Enter` apply, `Esc` clear)
- `z`: reset text/protocol/state filters
- `?`: open keyboard help overlay
- `c`: copy selected local address
- `m`: clear status message

## Build and quality checks

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT
