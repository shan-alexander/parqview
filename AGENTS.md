# AGENTS.md

Instructions and guidelines for AI agents working in this repository.

## Running & Testing the Application

> **CRITICAL**: Always use `./run.sh` to launch or test `parqview`.

`./run.sh` automatically manages:
1. Nix development shell environment resolution (if `nix` and `flake.nix` are present).
2. Proper Wayland display backend flags (and handles `PARQVIEW_X11=1` fallback if needed).
3. Building the release binary (`cargo build --release`) if it does not yet exist.

### Usage Examples
```bash
# Launch default viewer
./run.sh

# Launch viewer targeting a specific file or folder
./run.sh path/to/dataset.parquet
./run.sh path/to/data_folder/

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
