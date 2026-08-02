---
tags: [goal, vision, parquet, duckdb, multi-format]
node_type: goal
aliases: [parqview-vision, multi-format-parquet-reader]
---
# Lightweight Parquet Reader and Multi-Format Query Workbench

## Overview
`parqview` aims to be a lightweight, Rust-native, and blazing-fast Parquet file reader and data workbench. It enables users to effortlessly inspect data tables stored in Parquet files and run DuckDB queries to analyze datasets on the fly.

## Key Objectives
- **Blazing Fast Parquet Reader**: Instant loading and inspection of large Parquet datasets with high performance and low memory overhead.
- **Interactive DuckDB Querying**: Write and execute DuckDB SQL queries against loaded datasets with syntax highlighting and results grid display.
- **Multi-Format Data Support**: In addition to `.parquet`, support seamless browsing and querying of other popular tabular and structured data formats including `.csv`, `.tsv`, `.json`, `.jsonl`, `.xlsx`, and `.sql`.
- **Intuitive GUI Experience**: Feature-packed native interface including a folder navigation tree, schema inspection panel with column types, virtualized results grid, and an interactive SQL editor.
- **Lightweight & Self-Contained**: Pure Rust native UI avoiding bloated webview runtimes or complex multi-process web servers.
