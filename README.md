# parqview

Lightweight **egui** explorer for Parquet/CSV, powered by the system **DuckDB CLI**.

Inspired by [Duckling](https://github.com/l1xnan/duckling) (browse data + SQL), but avoids Tauri/WebView/AppImage/SDL — those failed on this NixOS box with EGL / SSL issues.

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

## Requirements

- Rust toolchain (or `nix develop` via `flake.nix`)
- `duckdb` on `PATH`
- Wayland or X11 display

## Installation

```bash
# Install directly via cargo:
cargo install --git https://github.com/shan-alexander/parqview

# Or from crates.io (once published):
cargo install parqview
```

## Usage

```bash
# Run in terminal to open current working directory:
parqview

# Target a specific dataset or folder:
parqview path/to/dataset.parquet
parqview path/to/data_folder/

# Local launcher script (handles Nix environment & display settings):
./run.sh [path/to/data_dir_or_file]

# Force X11 backend if needed:
PARQVIEW_X11=1 parqview
```

## Architecture

```
egui UI  ──SQL──►  duckdb CLI (-json)  ──►  parse JSON rows  ──►  table grid
```

No `libduckdb` link, no browser, no AppImage FHS.

Optional: `DUCKDB_BIN=/path/to/duckdb` to override the binary.
