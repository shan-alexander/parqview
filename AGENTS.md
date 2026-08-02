# AGENTS.md

Instructions and guidelines for AI agents working in this repository.

## Installation & Cargo Binary Setup

### Recommended: Cargo Install

```bash
# Install binary globally from git (use --force to pull latest commit):
cargo install --force --git https://github.com/shan-alexander/parqview

# Or install from local clone:
cargo install --force --path .
```

### Ensuring `~/.cargo/bin` is in PATH

If the `parqview` binary is not found in your terminal after installation:

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

## Running & Testing the Application

### CLI Invocation

`parqview` accepts positional path arguments:
```bash
# Open current working directory in parqview:
parqview

# Open a target directory tree (e.g., datalake root):
parqview /mnt/datalake/

# Open a specific file directly:
parqview path/to/dataset.parquet
```

### Development Launcher (`./run.sh`)

> **CRITICAL**: Use `./run.sh` to launch or test `parqview` during development if `parqview` is not yet installed in your `PATH`.

`./run.sh` automatically manages:
1. Nix development shell environment resolution (if `nix` and `flake.nix` are present).
2. Proper Wayland display backend flags (and handles `PARQVIEW_X11=1` fallback if needed).
3. Building the release binary (`cargo build --release`) if it does not yet exist.

Note: `parqview` requires `duckdb` installed on `PATH`.

```bash
# Launch default viewer via run.sh
./run.sh

# Launch viewer targeting a specific directory or file
./run.sh /mnt/datalake/
./run.sh path/to/dataset.parquet

# Force X11 backend if Wayland rendering issue occurs
PARQVIEW_X11=1 ./run.sh
```

## Build & Check Commands

```bash
# Fast compiler check
cargo check

# Standard build
cargo build

# Release build
cargo build --release
```

If operating on a Nix system:
```bash
nix develop -c cargo check
nix develop -c cargo build --release
```

## Architecture Summary

- `src/main.rs`: Main entry point configuring `eframe` window and launching native UI.
- `src/app.rs`: Main UI layout, table rendering, tab state, and user interactions.
- `src/duck.rs`: Subprocess interface to `duckdb -json` CLI for executing queries without native C++ bindings.
- `src/describe.rs`: Data profiling, statistics, coverage rates, and table grain detection.
- `src/derive.rs`: Automatic detection and derivation of currency and numeric string columns.
- `src/tree.rs`: Interactive folder tree filtering for supported extensions (`.parquet`, `.csv`, `.tsv`, `.json`, `.jsonl`, `.sql`).
- `src/theme.rs`: Dark theme styling, color tokens, and UI layout constants.
