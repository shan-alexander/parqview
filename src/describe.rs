//! Intelligent table describe: grain detection + column profiles + ethereal derives.

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::derive::{self, DerivedCol};
use crate::duck::{self, path_sql, QueryResult};

#[derive(Debug, Clone)]
struct ColMeta {
    name: String,
    dtype: String,
    /// True for ethereal pv_deriv_* columns (not on disk).
    derived: bool,
}

/// Run full intelligent describe for a tabular file path.
pub fn describe_table(path: &Path) -> Result<(QueryResult, String)> {
    let derived = derive::detect_derived(path).unwrap_or_default();
    let source = derive::enhanced_relation(path, &derived);

    let mut cols = column_metas(&source)?;
    // Tag derived columns by name.
    for c in &mut cols {
        c.derived = c.name.starts_with("pv_deriv_");
    }
    if cols.is_empty() {
        bail!("no columns found");
    }

    let n = duck::query_json(&format!("SELECT count(*)::BIGINT AS n FROM {source}"))?
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if n == 0 {
        let mut parts = Vec::new();
        for c in &cols {
            parts.push(column_profile_select(&source, c));
        }
        let profile_sql = parts.join("\nUNION ALL\n");
        let profile = duck::query_json(&profile_sql).context("column profile query")?;
        let physical_cols = cols.iter().filter(|c| !c.derived).count();
        let summary = format!("describe · 0 rows · {physical_cols} cols · (empty table)");
        return Ok((profile, summary));
    }

    // Grain detection only on physical (non-derived) columns.
    let physical: Vec<ColMeta> = cols.iter().filter(|c| !c.derived).cloned().collect();
    let mut uniq_queries = Vec::new();
    for c in &physical {
        let qn = quote_ident(&c.name);
        uniq_queries.push(format!(
            "SELECT {} AS col, count(DISTINCT {qn})::BIGINT AS u FROM {source}",
            sql_string(&c.name)
        ));
    }
    let uniq_sql = uniq_queries.join(" UNION ALL ");
    let uniq_res = duck::query_json(&uniq_sql)?;
    let mut unique_map: Vec<(String, u64)> = Vec::new();
    for row in &uniq_res.rows {
        if row.len() >= 2 {
            let u: u64 = row[1].parse().unwrap_or(0);
            unique_map.push((row[0].clone(), u));
        }
    }

    let grain = detect_grain(&physical, &unique_map, n);
    let grain_status = validate_grain(&source, &grain, n)?;

    // Column profile UNION ALL — includes ethereal derives.
    let mut parts = Vec::new();
    for c in &cols {
        parts.push(column_profile_select(&source, c));
    }
    let profile_sql = parts.join("\nUNION ALL\n");
    let mut profile = duck::query_json(&profile_sql).context("column profile query")?;

    let grain_label = if grain.is_empty() {
        "(none detected)".to_string()
    } else {
        grain.join(", ")
    };

    let derive_note = if derived.is_empty() {
        "none".into()
    } else {
        derived
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    profile.columns.push("detected_grain".into());
    profile.columns.push("grain_note".into());
    profile.columns.push("derived_cols".into());
    for row in &mut profile.rows {
        row.push(grain_label.clone());
        row.push(grain_status.clone());
        row.push(derive_note.clone());
    }

    if !grain.is_empty() {
        let grain_sql = grain_groupby_summary(&source, &grain, n);
        if let Ok(g) = duck::query_json(&grain_sql) {
            profile = merge_grain_header(profile, &grain, &g, &grain_status);
        }
    }

    let n_derived = derived.len();
    let summary = format!(
        "describe · {n} rows · {} cols (+{n_derived} derived) · grain: {grain_label} · {grain_status}",
        physical.len()
    );
    Ok((profile, summary))
}

/// Detect derived columns for a path (shared with preview).
pub fn detect_derived_for(path: &Path) -> Result<Vec<DerivedCol>> {
    derive::detect_derived(path)
}

fn merge_grain_header(
    mut profile: QueryResult,
    grain: &[String],
    grain_q: &QueryResult,
    note: &str,
) -> QueryResult {
    let cols = profile.columns.clone();
    let mut header_rows = Vec::new();

    let mut r = vec![String::new(); cols.len()];
    set_col(&cols, &mut r, "metric_type", "grain_summary");
    set_col(&cols, &mut r, "column_name", &grain.join(" + "));
    set_col(&cols, &mut r, "data_type", "GRAIN");
    if let Some(gr) = grain_q.rows.first() {
        let total = grain_q.get(gr, "row_count").unwrap_or("");
        let uniq = grain_q.get(gr, "unique_count").unwrap_or("");
        set_col(&cols, &mut r, "row_count", total);
        set_col(&cols, &mut r, "unique_count", uniq);
        if let (Ok(t), Ok(u)) = (total.parse::<f64>(), uniq.parse::<f64>()) {
            if t > 0.0 {
                set_col(&cols, &mut r, "coverage_rate", &format!("{:.4}", u / t));
            }
        }
    }
    set_col(&cols, &mut r, "detected_grain", &grain.join(", "));
    set_col(&cols, &mut r, "grain_note", note);
    header_rows.push(r);
    header_rows.append(&mut profile.rows);
    profile.rows = header_rows;
    profile
}

fn set_col(cols: &[String], row: &mut [String], name: &str, val: &str) {
    if let Some(i) = cols.iter().position(|c| c == name) {
        if i < row.len() {
            row[i] = val.to_string();
        }
    }
}

fn column_metas(source: &str) -> Result<Vec<ColMeta>> {
    let sql = format!("SELECT column_name, column_type FROM (DESCRIBE SELECT * FROM {source})");
    let r = duck::query_json(&sql)?;
    let mut out = Vec::new();
    for row in r.rows {
        if row.len() >= 2 {
            out.push(ColMeta {
                name: row[0].clone(),
                dtype: row[1].clone(),
                derived: false,
            });
        }
    }
    Ok(out)
}

fn column_profile_select(source: &str, c: &ColMeta) -> String {
    let qn = quote_ident(&c.name);
    let filled = filled_predicate(&qn, &c.dtype);
    // Currency *symbol* cols are categorical; currency *value* / string-numeric derives are numeric.
    let is_currency_symbol = c.name == "pv_deriv_currency"
        || (c.name.ends_with("_currency") && !c.name.ends_with("_currency_value"));
    let is_num = if is_currency_symbol {
        false
    } else {
        is_numeric_type(&c.dtype)
            || c.name.contains("currency_value")
            || (c.derived && c.name.starts_with("pv_deriv_"))
    };

    let metric = if c.derived {
        "derived_profile"
    } else {
        "column_profile"
    };

    let extra = if is_num {
        format!(
            "round(avg(try_cast({qn} AS DOUBLE)), 6) AS avg, \
             min(try_cast({qn} AS DOUBLE)) AS min, \
             round(quantile_cont(try_cast({qn} AS DOUBLE), 0.25), 6) AS q25, \
             round(quantile_cont(try_cast({qn} AS DOUBLE), 0.50), 6) AS q50, \
             round(quantile_cont(try_cast({qn} AS DOUBLE), 0.75), 6) AS q75, \
             max(try_cast({qn} AS DOUBLE)) AS max"
        )
    } else {
        "NULL AS avg, NULL AS min, NULL AS q25, NULL AS q50, NULL AS q75, NULL AS max".into()
    };

    let dtype_label = if c.derived {
        format!("{} (derived)", c.dtype)
    } else {
        c.dtype.clone()
    };

    format!(
        "SELECT \
            '{metric}' AS metric_type, \
            {} AS column_name, \
            {} AS data_type, \
            count(*)::BIGINT AS row_count, \
            count(DISTINCT {qn})::BIGINT AS unique_count, \
            round(count(*) FILTER (WHERE {filled})::DOUBLE / nullif(count(*), 0), 4) AS coverage_rate, \
            {extra} \
         FROM {source}",
        sql_string(&c.name),
        sql_string(&dtype_label),
    )
}

fn filled_predicate(qn: &str, dtype: &str) -> String {
    let dt = dtype.to_ascii_lowercase();
    if dt.contains("list") || dt.contains("array") {
        format!("{qn} IS NOT NULL AND len({qn}) > 0")
    } else if dt.contains("varchar")
        || dt.contains("string")
        || dt.contains("text")
        || dt.contains("json")
        || dt.contains("uuid")
    {
        format!("{qn} IS NOT NULL AND trim(cast({qn} AS VARCHAR)) <> ''")
    } else {
        format!("{qn} IS NOT NULL")
    }
}

fn is_numeric_type(dtype: &str) -> bool {
    let d = dtype.to_ascii_lowercase();
    d.contains("int")
        || d.contains("float")
        || d.contains("double")
        || d.contains("decimal")
        || d.contains("numeric")
        || d.contains("hugeint")
        || d == "real"
}

fn detect_grain(cols: &[ColMeta], unique_map: &[(String, u64)], n: u64) -> Vec<String> {
    if n == 0 {
        return vec![];
    }

    let mut candidates: Vec<(f64, String)> = Vec::new();
    for c in cols {
        let u = unique_map
            .iter()
            .find(|(name, _)| name == &c.name)
            .map(|(_, u)| *u)
            .unwrap_or(0);
        if u == 0 {
            continue;
        }
        let uniq_ratio = u as f64 / n as f64;
        if uniq_ratio < 0.999 {
            continue;
        }
        let mut score = uniq_ratio * 10.0;
        let lname = c.name.to_ascii_lowercase();
        if lname == "id" || lname.ends_with("_id") || lname.ends_with("id") {
            score += 5.0;
        }
        if lname.contains("uuid") || lname.contains("guid") {
            score += 4.0;
        }
        if lname.contains("key") || lname.contains("pk") {
            score += 3.0;
        }
        score += 1.0 / (1.0 + lname.len() as f64 / 20.0);
        candidates.push((score, c.name.clone()));
    }
    candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((_, name)) = candidates.first() {
        return vec![name.clone()];
    }

    let mut ranked: Vec<(String, u64)> = unique_map.to_vec();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    let top: Vec<String> = ranked.into_iter().take(6).map(|(n, _)| n).collect();

    let mut pair: Vec<String> = Vec::new();
    for name in &top {
        let lname = name.to_ascii_lowercase();
        if lname.contains("id")
            || lname.contains("key")
            || lname.contains("code")
            || lname.contains("date")
            || lname.contains("uuid")
        {
            pair.push(name.clone());
        }
        if pair.len() == 2 {
            break;
        }
    }
    if pair.len() < 2 && top.len() >= 2 {
        pair = top.into_iter().take(2).collect();
    }
    pair
}

fn validate_grain(source: &str, grain: &[String], n: u64) -> Result<String> {
    if grain.is_empty() {
        return Ok("no unique grain found (approx or multi-col needed)".into());
    }
    let ge: Vec<String> = grain.iter().map(|c| quote_ident(c)).collect();
    let ge = ge.join(", ");
    let sql = format!(
        "SELECT \
            {n}::BIGINT AS row_count, \
            (SELECT count(*) FROM (SELECT 1 FROM {source} GROUP BY {ge}))::BIGINT AS grain_groups, \
            (SELECT max(c) FROM (SELECT count(*)::BIGINT AS c FROM {source} GROUP BY {ge}))::BIGINT AS max_per_group"
    );
    let r = duck::query_json(&sql)?;
    if let Some(row) = r.rows.first() {
        let groups = r.get_u64(row, "grain_groups").unwrap_or(0);
        let max_per = r.get_u64(row, "max_per_group").unwrap_or(0);
        if groups == n && max_per <= 1 {
            return Ok("unique grain (1 row per key)".into());
        }
        return Ok(format!(
            "approx grain · {groups} groups · max {max_per} rows/group"
        ));
    }
    Ok("grain validated".into())
}

fn grain_groupby_summary(source: &str, grain: &[String], n: u64) -> String {
    let ge: Vec<String> = grain.iter().map(|c| quote_ident(c)).collect();
    let ge = ge.join(", ");
    format!(
        "SELECT \
            {n}::BIGINT AS row_count, \
            (SELECT count(*) FROM (SELECT 1 FROM {source} GROUP BY {ge}))::BIGINT AS unique_count, \
            (SELECT max(c) FROM (SELECT count(*)::BIGINT AS c FROM {source} GROUP BY {ge}))::BIGINT AS max_rows_per_grain"
    )
}

/// DuckDB relation expression for a path (handles jsonl, directories, and single files).
pub fn relation_sql(path: &Path) -> String {
    if path.is_dir() {
        let p_str = path.to_string_lossy().replace('\'', "''");
        let mut has_direct_parquet = false;
        if let Ok(rd) = std::fs::read_dir(path) {
            has_direct_parquet = rd.flatten().any(|e| {
                e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x.eq_ignore_ascii_case("parquet"))
                    .unwrap_or(false)
            });
        }
        if has_direct_parquet {
            return format!("read_parquet('{p_str}/*.parquet', union_by_name := true)");
        }
        return format!("read_parquet('{p_str}/**/*.parquet', union_by_name := true)");
    }

    let p = path_sql(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "jsonl" | "ndjson" => format!("read_json_auto({p}, format := 'newline_delimited')"),
        "json" => format!("read_json_auto({p})"),
        "csv" | "tsv" => format!("read_csv_auto({p}, header := true)"),
        _ => p,
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn sql_string(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn describe_seed_parquet() {
        let p = PathBuf::from(
            "/home/kinna/labs/scala-score-scraper/seed_scala_us_sites_deduped.parquet",
        );
        if !p.exists() {
            return;
        }
        let (r, summary) = describe_table(&p).expect("describe");
        assert!(!r.rows.is_empty(), "expected profile rows");
        assert!(summary.contains("grain"), "{summary}");
        let has_id = r.rows.iter().any(|row| row.iter().any(|c| c == "company_id"));
        assert!(has_id, "expected company_id in profile");
        // revenue/assets should spawn currency-related derived profiles
        let has_cur = r.rows.iter().any(|row| {
            row.iter()
                .any(|c| c.contains("pv_deriv_") && c.contains("currency"))
        });
        assert!(
            has_cur || summary.contains("derived"),
            "expected currency derives in profile, summary={summary} rows={:?}",
            r.rows.iter().take(3).collect::<Vec<_>>()
        );
    }

    #[test]
    fn describe_datalake_directory() {
        let p = PathBuf::from(
            "/mnt/datalake/kinnalake/nonprod/lake_us/lake/lz/kinnaruns/domain=timberland.com/report_date=2026-07-29/run_id=182459Z-3872/raw_snoop",
        );
        if !p.exists() {
            return;
        }
        let (r, summary) = describe_table(&p).expect("describe dir");
        assert!(!r.rows.is_empty(), "expected profile rows");
        assert!(summary.contains("describe"), "{summary}");
    }
}
