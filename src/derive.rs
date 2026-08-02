//! Ethereal derived columns: currency parsing + string-numeric casts.

use std::path::Path;

use anyhow::Result;

use crate::describe::relation_sql;
use crate::duck;

/// Kind of ethereal column we materialize in a SELECT wrapper.
#[derive(Debug, Clone)]
pub enum DerivedKind {
    /// ISO/symbol currency code extracted from a money string column.
    CurrencySymbol { source: String },
    /// Numeric amount parsed from a money string (handles K/M/B/T suffixes).
    CurrencyValue { source: String },
    /// Plain numeric value parsed from a varchar that is not currency.
    NumericFromString { source: String },
}

#[derive(Debug, Clone)]
pub struct DerivedCol {
    /// Output name, e.g. `pv_deriv_currency` or `pv_deriv_revenue`.
    pub name: String,
    pub kind: DerivedKind,
    /// Short human reason shown in status / describe.
    pub note: String,
}

/// Detect varchar columns that are currency or string-numerics; return derived specs.
pub fn detect_derived(path: &Path) -> Result<Vec<DerivedCol>> {
    let base = relation_sql(path);
    let cols = varchar_columns(&base)?;
    if cols.is_empty() {
        return Ok(vec![]);
    }

    let mut currency_sources: Vec<String> = Vec::new();
    let mut numeric_sources: Vec<String> = Vec::new();

    for col in &cols {
        let qn = quote_ident(col);
        // Sample filled rows (up to 500) for classification.
        let cur_pred = currency_match_sql("v");
        let num_pred = plain_numeric_match_sql("v");
        let sql = format!(
            "WITH s AS (\
                SELECT cast({qn} AS VARCHAR) AS v FROM {base} \
                WHERE {qn} IS NOT NULL AND trim(cast({qn} AS VARCHAR)) <> '' \
                LIMIT 500\
             ) \
             SELECT \
               count(*)::BIGINT AS n, \
               count(*) FILTER (WHERE {cur_pred})::BIGINT AS n_cur, \
               count(*) FILTER (WHERE {num_pred})::BIGINT AS n_num \
             FROM s"
        );
        let r = duck::query_json(&sql)?;
        let (n, n_cur, n_num) = match r.rows.first() {
            Some(row) if row.len() >= 3 => (
                row[0].parse::<f64>().unwrap_or(0.0),
                row[1].parse::<f64>().unwrap_or(0.0),
                row[2].parse::<f64>().unwrap_or(0.0),
            ),
            _ => continue,
        };
        if n < 5.0 {
            continue;
        }
        let cur_rate = n_cur / n;
        let num_rate = n_num / n;

        // Name boost for money-ish columns.
        let lname = col.to_ascii_lowercase();
        let money_name = ["revenue", "price", "cost", "amount", "sales", "assets", "income", "profit", "fee", "value"]
            .iter()
            .any(|k| lname.contains(k));

        let cur_thresh = if money_name { 0.75 } else { 0.90 };
        let num_thresh = if money_name { 0.85 } else { 0.92 };

        if cur_rate >= cur_thresh {
            currency_sources.push(col.clone());
        } else if num_rate >= num_thresh && cur_rate < 0.5 {
            // Prefer currency classification when both fire.
            numeric_sources.push(col.clone());
        }
    }

    let mut out = Vec::new();

    // Currency derived columns.
    match currency_sources.as_slice() {
        [] => {}
        [only] => {
            out.push(DerivedCol {
                name: "pv_deriv_currency".into(),
                kind: DerivedKind::CurrencySymbol {
                    source: only.clone(),
                },
                note: format!("currency symbol from `{only}`"),
            });
            out.push(DerivedCol {
                name: "pv_deriv_currency_value".into(),
                kind: DerivedKind::CurrencyValue {
                    source: only.clone(),
                },
                note: format!("numeric amount from `{only}` (K/M/B/T aware)"),
            });
        }
        many => {
            for src in many {
                let safe = sanitize_ident(src);
                out.push(DerivedCol {
                    name: format!("pv_deriv_{safe}_currency"),
                    kind: DerivedKind::CurrencySymbol {
                        source: src.clone(),
                    },
                    note: format!("currency symbol from `{src}`"),
                });
                out.push(DerivedCol {
                    name: format!("pv_deriv_{safe}_currency_value"),
                    kind: DerivedKind::CurrencyValue {
                        source: src.clone(),
                    },
                    note: format!("numeric amount from `{src}` (K/M/B/T aware)"),
                });
            }
        }
    }

    for src in numeric_sources {
        // Skip if this column already treated as currency.
        if currency_sources.iter().any(|c| c == &src) {
            continue;
        }
        let safe = sanitize_ident(&src);
        out.push(DerivedCol {
            name: format!("pv_deriv_{safe}"),
            kind: DerivedKind::NumericFromString {
                source: src.clone(),
            },
            note: format!("numeric cast from varchar `{src}`"),
        });
    }

    Ok(out)
}

/// Wrap base relation with ethereal derived columns: `(SELECT *, expr AS name, ... FROM base) t`
pub fn enhanced_relation(path: &Path, derived: &[DerivedCol]) -> String {
    let base = relation_sql(path);
    if derived.is_empty() {
        return base;
    }
    let mut selects = vec!["*".to_string()];
    for d in derived {
        selects.push(format!("{} AS {}", d.sql_expr(), quote_ident(&d.name)));
    }
    format!(
        "(SELECT {} FROM {base})",
        selects.join(", ")
    )
}

impl DerivedCol {
    pub fn sql_expr(&self) -> String {
        match &self.kind {
            DerivedKind::CurrencySymbol { source } => {
                let q = quote_ident(source);
                // Prefer explicit symbols; fall back to ISO codes.
                format!(
                    "CASE \
                        WHEN regexp_matches(trim(cast({q} AS VARCHAR)), '€') THEN '€' \
                        WHEN regexp_matches(trim(cast({q} AS VARCHAR)), '\\$') THEN '$' \
                        WHEN regexp_matches(trim(cast({q} AS VARCHAR)), '£') THEN '£' \
                        WHEN regexp_matches(trim(cast({q} AS VARCHAR)), '¥') THEN '¥' \
                        WHEN regexp_matches(trim(cast({q} AS VARCHAR)), '₹') THEN '₹' \
                        WHEN regexp_matches(upper(trim(cast({q} AS VARCHAR))), '\\bEUR\\b') THEN 'EUR' \
                        WHEN regexp_matches(upper(trim(cast({q} AS VARCHAR))), '\\bUSD\\b') THEN 'USD' \
                        WHEN regexp_matches(upper(trim(cast({q} AS VARCHAR))), '\\bGBP\\b') THEN 'GBP' \
                        WHEN regexp_matches(upper(trim(cast({q} AS VARCHAR))), '\\bJPY\\b') THEN 'JPY' \
                        ELSE regexp_extract(trim(cast({q} AS VARCHAR)), '([€$£¥₹])', 1) \
                     END"
                )
            }
            DerivedKind::CurrencyValue { source } => currency_value_expr(&quote_ident(source)),
            DerivedKind::NumericFromString { source } => {
                let q = quote_ident(source);
                format!(
                    "try_cast(replace(replace(trim(cast({q} AS VARCHAR)), ',', ''), ' ', '') AS DOUBLE)"
                )
            }
        }
    }

    pub fn data_type_label(&self) -> &'static str {
        match self.kind {
            DerivedKind::CurrencySymbol { .. } => "VARCHAR (derived)",
            DerivedKind::CurrencyValue { .. } | DerivedKind::NumericFromString { .. } => {
                "DOUBLE (derived)"
            }
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self.kind,
            DerivedKind::CurrencyValue { .. } | DerivedKind::NumericFromString { .. }
        )
    }
}

/// Parse money strings like €716.9B, $1,234.5M, 12.5k into DOUBLE.
fn currency_value_expr(qn: &str) -> String {
    // 1) strip symbols/codes/spaces/commas → keep digits, dot, sign, and trailing scale letter
    // 2) extract mantissa + optional K/M/B/T
    format!(
        "(\
            try_cast(\
                regexp_extract(\
                    regexp_replace(trim(cast({qn} AS VARCHAR)), '[€$£¥₹,\\s]', '', 'g'), \
                    '([-+]?[0-9]*\\.?[0-9]+)', 1\
                ) AS DOUBLE\
            ) * CASE upper(coalesce(\
                nullif(regexp_extract(\
                    regexp_replace(trim(cast({qn} AS VARCHAR)), '[€$£¥₹,\\s]', '', 'g'), \
                    '([KkMmBbTt])\\s*$', 1\
                ), ''), \
                ''\
            )) \
                WHEN 'K' THEN 1e3 \
                WHEN 'M' THEN 1e6 \
                WHEN 'B' THEN 1e9 \
                WHEN 'T' THEN 1e12 \
                ELSE 1.0 \
            END\
        )"
    )
}

/// SQL boolean: value looks like currency (symbol/code + number, optional scale).
fn currency_match_sql(col: &str) -> String {
    // col is already an identifier or alias like v
    format!(
        "( \
            regexp_matches(trim(cast({col} AS VARCHAR)), '[€$£¥₹]') \
            OR regexp_matches(upper(trim(cast({col} AS VARCHAR))), '\\b(USD|EUR|GBP|JPY|INR)\\b') \
        ) AND regexp_matches(trim(cast({col} AS VARCHAR)), '[0-9]')"
    )
}

/// SQL boolean: value is a plain number (optional commas/sign/decimals), not currency.
fn plain_numeric_match_sql(col: &str) -> String {
    format!(
        "try_cast(replace(replace(trim(cast({col} AS VARCHAR)), ',', ''), ' ', '') AS DOUBLE) IS NOT NULL \
         AND NOT regexp_matches(trim(cast({col} AS VARCHAR)), '[€$£¥₹A-Za-z]')"
    )
}

fn varchar_columns(source: &str) -> Result<Vec<String>> {
    let sql = format!(
        "SELECT column_name, column_type FROM (DESCRIBE SELECT * FROM {source})"
    );
    let r = duck::query_json(&sql)?;
    let mut out = Vec::new();
    for row in r.rows {
        if row.len() < 2 {
            continue;
        }
        let dt = row[1].to_ascii_lowercase();
        if dt.contains('[')
            || dt.contains("list")
            || dt.contains("array")
            || dt.contains("struct")
            || dt.contains("map")
        {
            continue;
        }
        if dt.contains("varchar")
            || dt.contains("string")
            || dt.contains("text")
            || dt.contains("json")
        {
            out.push(row[0].clone());
        }
    }
    Ok(out)
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn sanitize_ident(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if s.is_empty() {
        s = "col".into();
    }
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s = format!("c_{s}");
    }
    s
}

/// Public helper for tests / preview SELECT list.
pub fn select_star_with_derived(path: &Path, derived: &[DerivedCol], limit: u64) -> String {
    let rel = enhanced_relation(path, derived);
    format!("SELECT * FROM {rel} LIMIT {limit}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_revenue_currency_on_seed() {
        let p = PathBuf::from(
            "/home/kinna/labs/scala-score-scraper/seed_scala_us_sites_deduped.parquet",
        );
        if !p.exists() {
            return;
        }
        let d = detect_derived(&p).expect("detect");
        assert!(
            d.iter().any(|c| c.name == "pv_deriv_currency"
                || c.name.contains("currency_value")
                || c.name.contains("revenue")),
            "expected currency derives, got: {:?}",
            d.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        // Smoke the enhanced query returns derived cols.
        let sql = select_star_with_derived(&p, &d, 3);
        let r = duck::query_json(&sql).expect("query");
        assert!(
            r.columns.iter().any(|c| c.starts_with("pv_deriv_")),
            "cols={:?}",
            r.columns
        );
    }
}
