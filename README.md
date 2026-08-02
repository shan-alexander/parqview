# parqview

Lightweight **egui** explorer for Parquet/CSV, powered by the system **DuckDB CLI**.

Inspired by [Duckling](https://github.com/l1xnan/duckling) (browse data + SQL), but avoids Tauri/WebView/AppImage/SDL — those on Nix OS with EGL / SSL issues.

## Why this exists

| Tool | Problem here |
|------|----------------|
| DuckDB Local UI | Binds `[::1]` only + fetches UI from `ui.duckdb.org` (SSL fail) |
| Duckling AppImage | `appimage-run` → EGL_BAD_PARAMETER |
| **parqview** | Native egui (OpenGL/glow) + `duckdb -json` subprocess |

## Features

- **Folder tree** — only shows `.parquet` / `.csv` / `.tsv` / `.json` / `.jsonl` / `.sql` (+ dirs to navigate)
- **Schema panel** — column names + types (independent scroll)
- **Results grid** — own scroll via TableBuilder
- **SQL editor** — own scroll; Run / Ctrl+Enter; `.sql` files load into the editor
- **Wayland-first** (set `PARQVIEW_X11=1` only if you need X11 fallback)
- Dark aesthetic polish (accent panels, floating scrollbars)

## Requirements & Platform Support

- **Linux (NixOS / Ubuntu / Arch)**: Wayland or X11 display + `duckdb` CLI on `PATH`.
- **macOS (Apple Silicon & Intel)**: Native Cocoa/Metal GUI + `duckdb` CLI on `PATH` (`brew install duckdb`).

## Installation

### Recommended: Cargo Install

Install directly via `cargo`:

```bash
# Install directly from GitHub (use --force to ensure latest commit is compiled):
cargo install --force --git https://github.com/shan-alexander/parqview

# Or from crates.io (once published):
cargo install --force parqview
```

### Adding `~/.cargo/bin` to PATH

If `parqview` is not recognized after installation, ensure `~/.cargo/bin` is in your `PATH`:

```bash
# Temporary (bash/zsh):
export PATH="$HOME/.cargo/bin:$PATH"

# Temporary (fish shell):
set -gx PATH $HOME/.cargo/bin $PATH

# Permanent fix (fish shell):
fish_add_path ~/.cargo/bin

# Permanent fix (bash):
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc

# Permanent fix (zsh):
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

## Usage

```bash
# Open current working directory in parqview:
parqview

# Open a target directory (e.g. datalake folder):
parqview /mnt/datalake/

# Open a specific file directly:
parqview path/to/dataset.parquet

# Local launcher script (handles Nix environment & display settings):
./run.sh [path/to/data_dir_or_file]

# Force X11 backend if Wayland issues occur:
PARQVIEW_X11=1 parqview
```

## Architecture

```
egui UI  ──SQL──►  duckdb CLI (-json)  ──►  parse JSON rows  ──►  table grid
```

No `libduckdb` link, no browser, no AppImage FHS.

Optional: `DUCKDB_BIN=/path/to/duckdb` to override the binary.
