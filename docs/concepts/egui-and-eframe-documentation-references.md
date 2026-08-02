---
tags: [reference, egui, eframe, docs, rust, duckdb]
node_type: reference
aliases: [egui-docs, egui-references]
---
# egui and eframe Documentation References

Essential documentation, crate references, guides, and repositories for developing `parqview` with `egui`, `eframe`, and `duckdb`.

## Official Framework Documentation

- **egui Documentation (docs.rs)**: https://docs.rs/egui/latest/egui/
  - Main API reference for `egui` widgets, layouts, styles, and context (`egui::Context`, `egui::Ui`).
- **eframe Documentation (docs.rs)**: https://docs.rs/eframe/latest/eframe/
  - Official framework for running `egui` applications natively on desktop (Glow/OpenGL, wgpu, Wayland/X11).
- **egui Interactive Web Demo**: https://www.egui.rs/
  - Live demo showcasing widget capabilities, table styling, themes, and painter tools.
- **egui GitHub Repository**: https://github.com/emilk/egui
  - Source code, issues, feature requests, and official examples.

## Key Table & Grid Layout Components

- **`egui_extras::TableBuilder` (docs.rs)**: https://docs.rs/egui_extras/latest/egui_extras/struct.TableBuilder.html
  - High-performance, virtualized data grid rendering for large datasets (renders only visible rows for instant UI responsiveness).
- **`egui::Grid` Layouts**: https://docs.rs/egui/latest/egui/struct.Grid.html
  - 2D grid layout for aligned labels, options, and form controls.

## Subprocess & Data Interop References

- **DuckDB CLI Documentation**: https://duckdb.org/docs/api/cli
  - Reference for `duckdb` command-line flags, specifically `-json` and query execution parameters.
- **DuckDB Parquet Reader Reference**: https://duckdb.org/docs/data/parquet/overview.html
  - Details on reading `.parquet` files and querying multi-file dataset globs.
