# port-finder

Cross-platform CLI + TUI for discovering and reclaiming busy ports quickly.

## Why

When local development breaks because a port is already in use, `port-finder` gives you fast answers and one-command cleanup.

## Features

- List active ports with process details
- Find what owns a specific port
- Kill process by port
- Check whether a port is available
- Scan ranges for available/in-use ports
- Interactive TUI mode (`pf`)
- Selection persistence across refresh and auto-refresh
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
pf kill 3000
pf check 8080
pf scan 3000-4000
```

## Command reference

- `pf`: launch interactive TUI
- `pf list [--all]`: list active ports (`--all` includes non-listen states)
- `pf find <port>`: show process details for one port
- `pf kill <port> [--force]`: terminate process on a port
- `pf check <port>`: return availability status
- `pf scan <start-end>`: scan a range like `3000-4000`

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
