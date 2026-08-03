//! Thin DuckDB CLI backend — uses system `duckdb` (no native lib link / no SSL UI).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::describe::relation_sql;

#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub row_count_hint: Option<u64>,
    pub elapsed_ms: u128,
    pub sql: String,
}

pub fn duckdb_bin() -> PathBuf {
    std::env::var_os("DUCKDB_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("duckdb"))
}

pub fn check_duckdb_version() -> Result<String> {
    let out = Command::new(duckdb_bin())
        .arg("--version")
        .output()
        .with_context(|| {
            format!(
                "failed to spawn duckdb ({})",
                duckdb_bin().display()
            )
        })?;
    if !out.status.success() {
        bail!("duckdb --version failed with status {}", out.status);
    }
    let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(ver)
}

/// Run SQL; expect JSON array of objects (duckdb -json).
pub fn query_json(sql: &str) -> Result<QueryResult> {
    let start = std::time::Instant::now();
    let out = Command::new(duckdb_bin())
        .args(["-json", "-c", sql])
        .output()
        .with_context(|| {
            format!(
                "failed to spawn duckdb ({}). Is duckdb on PATH?",
                duckdb_bin().display()
            )
        })?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if !out.status.success() {
        let msg = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("duckdb exited {}", out.status)
        };
        bail!("{msg}");
    }

    if stdout.is_empty() {
        let cols = describe_sql_columns(sql);
        return Ok(QueryResult {
            columns: cols,
            rows: vec![],
            row_count_hint: Some(0),
            elapsed_ms: start.elapsed().as_millis(),
            sql: sql.to_string(),
        });
    }

    let val: Value =
        serde_json::from_str(&stdout).with_context(|| format!("parse duckdb JSON:\n{stdout}"))?;

    let arr = match val {
        Value::Array(a) => a,
        other => bail!("expected JSON array from duckdb, got {other}"),
    };

    if arr.is_empty() {
        let cols = describe_sql_columns(sql);
        return Ok(QueryResult {
            columns: cols,
            rows: vec![],
            row_count_hint: Some(0),
            elapsed_ms: start.elapsed().as_millis(),
            sql: sql.to_string(),
        });
    }

    let mut columns: Vec<String> = Vec::new();
    if let Value::Object(map) = &arr[0] {
        columns = map.keys().cloned().collect();
    }

    let mut rows = Vec::with_capacity(arr.len());
    for item in &arr {
        let Value::Object(map) = item else {
            bail!("expected object rows in JSON array");
        };
        let mut row = Vec::with_capacity(columns.len());
        for c in &columns {
            let cell = map.get(c).map(json_cell).unwrap_or_default();
            row.push(pretty_cell(&cell));
        }
        rows.push(row);
    }

    let n = rows.len() as u64;
    Ok(QueryResult {
        columns,
        rows,
        row_count_hint: Some(n),
        elapsed_ms: start.elapsed().as_millis(),
        sql: sql.to_string(),
    })
}

fn json_cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Pretty-print JSON object/array strings; leave other cells unchanged.
fn pretty_cell(s: &str) -> String {
    let t = s.trim();
    if (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']')) {
        if let Ok(v) = serde_json::from_str::<Value>(t) {
            if let Ok(pretty) = serde_json::to_string_pretty(&v) {
                return pretty;
            }
        }
    }
    s.to_string()
}

pub fn preview_file(path: &Path, limit: u64) -> Result<QueryResult> {
    let rel = relation_sql(path);
    let sql = format!("SELECT * FROM {rel} LIMIT {limit}");
    query_json(&sql)
}

pub fn describe_file(path: &Path) -> Result<QueryResult> {
    let rel = relation_sql(path);
    let sql = format!("SELECT column_name, column_type FROM (DESCRIBE SELECT * FROM {rel})");
    query_json(&sql)
}

pub fn count_file(path: &Path) -> Result<u64> {
    let rel = relation_sql(path);
    let sql = format!("SELECT count(*)::BIGINT AS n FROM {rel}");
    let r = query_json(&sql)?;
    let n = r
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    Ok(n)
}

/// Quote a filesystem path for DuckDB string literal.
pub fn path_sql(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\'', "''");
    format!("'{s}'")
}

pub fn default_sql_for(path: &Path) -> String {
    format!("SELECT * FROM {} LIMIT 100", relation_sql(path))
}

impl QueryResult {
    /// TSV for clipboard (headers + rows). Newlines in cells become spaces.
    pub fn to_tsv(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.columns.join("\t"));
        out.push('\n');
        for row in &self.rows {
            let line: Vec<String> = row
                .iter()
                .map(|c| c.replace(['\t', '\n', '\r'], " "))
                .collect();
            out.push_str(&line.join("\t"));
            out.push('\n');
        }
        out
    }

    /// Look up a cell by column name (stable across DuckDB JSON key ordering).
    pub fn get<'a>(&self, row: &'a [String], col: &str) -> Option<&'a str> {
        let i = self.columns.iter().position(|c| c == col)?;
        row.get(i).map(|s| s.as_str())
    }

    pub fn get_u64(&self, row: &[String], col: &str) -> Option<u64> {
        self.get(row, col)?.parse().ok()
    }
}

fn describe_sql_columns(sql: &str) -> Vec<String> {
    let desc_sql = format!("SELECT column_name FROM (DESCRIBE ({sql}))");
    let out = Command::new(duckdb_bin())
        .args(["-json", "-c", &desc_sql])
        .output();
    let Ok(out) = out else { return vec![] };
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if stdout.is_empty() {
        return vec![];
    }
    let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(&stdout) else {
        return vec![];
    };
    let mut cols = Vec::new();
    for item in arr {
        if let Value::Object(map) = item {
            if let Some(Value::String(c)) = map.get("column_name") {
                cols.push(c.clone());
            }
        }
    }
    cols
}
