---
tags: [adr, gui, egui, tauri, architecture]
node_type: adr
aliases: [adr-0001, egui-over-tauri]
---
# 0001 Use egui as Preferred GUI

## Status

Accepted

## Context

`parqview` requires a lightweight, fast, cross-platform graphical user interface to inspect tabular data files (Parquet, JSONL, JSON, CSV, TSV, XLSX) and run DuckDB SQL queries interactively.

When evaluating GUI frameworks for Rust, several GUI options exist in the Rust ecosystem:
1. **Tauri (WebView-based)**: Bundles a Rust backend with system WebViews (WebKitGTK/wry on Linux, Webview2 on Windows, WebKit on macOS) driving a web frontend (HTML/JS/CSS).
2. **Iced / Slint / Dioxus (Retained / Reactive UI frameworks)**: Declarative/retained GUI engines.
3. **egui (Immediate Mode GUI)**: Pure Rust immediate-mode GUI rendered via OpenGL (`glow`), WebGPU (`wgpu`), or DirectX.

### Issues with WebViews & Tauri on Target Systems
- **System Dependencies & Driver Crashes**: WebViews on Linux (WebKitGTK/WRY) frequently suffer from environment-specific issues such as `EGL_BAD_PARAMETER`, hardware acceleration failures under Wayland/NixOS, and missing runtime libraries.
- **Resource Footprint & Overhead**: Tauri webviews incur high memory overhead (~100–300 MB per window) and IPC serialization bottlenecks when transferring large JSON query results between Rust and JavaScript.
- **Network / SSL Complications**: Embedded web servers or local binding (e.g., DuckDB web UI or local RPC bridges) often trigger SSL certificate mismatches or loopback binding failures (e.g., `[::1]` binding issues).

## Decision

We chose **`egui`** (via `eframe` and `glow`/`wgpu`) as the standard, primary GUI engine for `parqview`.

`parqview` will directly render its interface using pure Rust native graphics bindings without using WebViews, browser engines, or external web server processes.

## Rationale

1. **Pure Rust Native Stack**: `egui` compiles down to native machine code with zero external runtime GUI dependencies (no WebKitGTK, webkit2gtk, or Node runtime).
2. **Zero IPC Overhead**: Data table rows, schemas, and query results stay strictly within Rust memory, allowing direct, zero-copy rendering into UI widgets without JSON serialization over IPC boundaries.
3. **Virtualized High-Performance Grid Rendering**: Immediate mode GUI coupled with `egui_extras::TableBuilder` allows `parqview` to render hundreds of thousands of table rows smoothly by only drawing visible rows on screen.
4. **Instant Startup & Low Resource Usage**: Minimal binary size, under 15–30 MB memory footprint, and sub-100ms startup times.
5. **Wayland & Linux Native Compatibility**: Excellent support for modern Linux graphics stacks (Wayland, X11 fallback via `PARQVIEW_X11`), avoiding WebKitGTK EGL driver errors.
6. **Subprocess Isolation**: Interfacing with system `duckdb -json` via direct CLI subprocesses keeps query execution robust and isolated without needing C bindings or complex local HTTP servers.

## Consequences

- **Immediate Mode Mental Model**: UI layouts must be specified on each frame. State management is kept simple and explicit in Rust structs.
- **Custom Styling & Layouts**: While `egui` has standard themes and dark modes, pixel-perfect custom CSS is not available; styling is performed using `egui` panels, styles, and custom painters (which provides a unified, dark aesthetic across all platforms).
