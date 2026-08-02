//! Main egui application — folder tree + schema + results + SQL, independent scrolls.

use std::path::PathBuf;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::derive::{self, DerivedCol};
use crate::describe;
use crate::duck::{self, QueryResult};
use crate::theme;
use crate::tree::{FolderTree, TreeAction};

pub struct ParqApp {
    tree: FolderTree,
    open_path: Option<PathBuf>,
    total_rows: Option<u64>,
    schema: Option<QueryResult>,
    result: Option<QueryResult>,
    /// Ethereal derived columns for the open data file.
    derived: Vec<DerivedCol>,
    sql: String,
    error: Option<String>,
    status: String,
    preview_limit: u64,
    show_schema: bool,
    theme_applied: bool,
    clipboard_flash: Option<String>,
}

impl Default for ParqApp {
    fn default() -> Self {
        Self {
            tree: FolderTree::default(),
            open_path: None,
            total_rows: None,
            schema: None,
            result: None,
            derived: Vec::new(),
            sql: String::new(),
            error: None,
            status: "Open a folder · parquet · csv · json · jsonl · sql".into(),
            preview_limit: 100,
            show_schema: true,
            theme_applied: false,
            clipboard_flash: None,
        }
    }
}

impl ParqApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut app = Self::default();
        theme::apply_theme(&cc.egui_ctx);
        app.theme_applied = true;

        if let Some(p) = initial {
            if p.is_dir() {
                app.tree.set_root(p.clone());
                if let Some(first) = find_first_tabular(&p, 0) {
                    app.open_data_file(first);
                }
            } else if p.is_file() {
                if let Some(parent) = p.parent() {
                    app.tree.set_root(parent.to_path_buf());
                    app.tree.selected = Some(p.clone());
                }
                if FolderTree::is_sql(&p) {
                    // CLI arg: allow loading sql from argv
                    app.load_sql_file(p);
                } else if FolderTree::is_tabular(&p) {
                    app.open_data_file(p);
                }
            }
        } else {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            if cwd.is_dir() {
                app.tree.set_root(cwd.clone());
                if let Some(first) = find_first_tabular(&cwd, 0) {
                    app.open_data_file(first);
                }
            }
        }
        app
    }

    fn load_sql_file(&mut self, path: PathBuf) {
        self.error = None;
        match std::fs::read_to_string(&path) {
            Ok(s) => {
                self.tree.selected = Some(path.clone());
                self.sql = s;
                self.status = format!(
                    "loaded SQL · {}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
                );
            }
            Err(e) => self.error = Some(format!("read sql: {e}")),
        }
    }

    fn open_data_file(&mut self, path: PathBuf) {
        self.error = None;
        if !path.exists() {
            self.error = Some(format!("not found: {}", path.display()));
            return;
        }
        if !FolderTree::is_tabular(&path) {
            self.error = Some(format!(
                "unsupported type (want parquet/csv/json/jsonl): {}",
                path.display()
            ));
            return;
        }

        self.open_path = Some(path.clone());
        self.tree.selected = Some(path.clone());

        // Detect ethereal derives (currency / string-numeric).
        self.derived = derive::detect_derived(&path).unwrap_or_default();
        self.sql = derive::select_star_with_derived(&path, &self.derived, self.preview_limit);

        match duck::count_file(&path) {
            Ok(n) => self.total_rows = Some(n),
            Err(e) => {
                self.total_rows = None;
                self.error = Some(format!("count: {e:#}"));
            }
        }

        // Schema of enhanced relation (includes derived col types when present).
        let schema_sql = format!(
            "SELECT column_name, column_type FROM (DESCRIBE SELECT * FROM {})",
            derive::enhanced_relation(&path, &self.derived)
        );
        match duck::query_json(&schema_sql) {
            Ok(s) => self.schema = Some(s),
            Err(e) => {
                self.schema = None;
                self.error = Some(format!("schema: {e:#}"));
            }
        }

        match duck::query_json(&self.sql) {
            Ok(r) => {
                let kind = if FolderTree::is_jsonl(&path) {
                    "jsonl"
                } else {
                    path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("data")
                };
                let deriv = if self.derived.is_empty() {
                    String::new()
                } else {
                    format!("  ·  +{} derived", self.derived.len())
                };
                self.status = format!(
                    "● {} ({kind})  ·  {} cols  ·  {} shown{}{deriv}  ·  {} ms",
                    path.file_name().and_then(|s| s.to_str()).unwrap_or("file"),
                    r.columns.len(),
                    r.rows.len(),
                    self.total_rows
                        .map(|n| format!(" / {n}"))
                        .unwrap_or_default(),
                    r.elapsed_ms
                );
                self.result = Some(r);
            }
            Err(e) => {
                self.result = None;
                self.error = Some(format!("preview: {e:#}"));
            }
        }
    }

    fn run_sql(&mut self) {
        self.error = None;
        let sql = self.sql.trim();
        if sql.is_empty() {
            self.error = Some("SQL is empty".into());
            return;
        }
        match duck::query_json(sql) {
            Ok(r) => {
                self.status = format!(
                    "✓ query  ·  {} cols  ·  {} rows  ·  {} ms",
                    r.columns.len(),
                    r.rows.len(),
                    r.elapsed_ms
                );
                self.result = Some(r);
            }
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }

    fn run_describe(&mut self) {
        self.error = None;
        let Some(path) = self.open_path.clone() else {
            self.error = Some("open a data file first (parquet/csv/json/jsonl)".into());
            return;
        };
        if !FolderTree::is_tabular(&path) {
            self.error = Some("Describe table needs a tabular file open".into());
            return;
        }
        self.status = "running intelligent describe…".into();
        match describe::describe_table(&path) {
            Ok((r, summary)) => {
                self.status = format!("✓ {summary}  ·  {} ms", r.elapsed_ms);
                // Put the describe SQL-ish note into status; results show profile.
                self.result = Some(r);
            }
            Err(e) => self.error = Some(format!("describe: {e:#}")),
        }
    }

    fn copy_results(&mut self, ctx: &egui::Context) {
        let Some(r) = &self.result else {
            self.error = Some("no results to copy".into());
            return;
        };
        let tsv = r.to_tsv();
        ctx.copy_text(tsv);
        self.clipboard_flash = Some(format!(
            "copied {}×{} TSV to clipboard",
            r.columns.len(),
            r.rows.len()
        ));
        self.status = self.clipboard_flash.clone().unwrap_or_default();
    }
}

impl eframe::App for ParqApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            theme::apply_theme(ctx);
            self.theme_applied = true;
        }

        // ── Top bar ──────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top")
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("parqview")
                            .color(theme::ACCENT)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("parquet · csv · json · jsonl · sql")
                            .small()
                            .color(theme::TEXT_MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(&mut self.show_schema, "schema");
                        ui.add(
                            egui::DragValue::new(&mut self.preview_limit)
                                .range(1..=50_000)
                                .prefix("limit ")
                                .speed(5.0),
                        );
                    });
                });
            });

        // ── Status ───────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status")
            .exact_height(28.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 4))
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(err) = &self.error {
                        ui.colored_label(theme::ERR, format!("✗ {err}"));
                    } else {
                        ui.colored_label(theme::OK, &self.status);
                    }
                });
            });

        // ── SQL editor ───────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("sql_panel")
            .resizable(true)
            .default_height(180.0)
            .min_height(100.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    theme::section_label(ui, "SQL");
                    ui.add_space(8.0);
                    let run = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("▶  Run").color(theme::BG).strong(),
                            )
                            .fill(theme::ACCENT)
                            .corner_radius(6),
                        )
                        .on_hover_text("Ctrl+Enter");
                    if run.clicked()
                        || ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter))
                    {
                        self.run_sql();
                    }
                    if ui
                        .button("Describe table")
                        .on_hover_text(
                            "Grain detection + column profiles (coverage, uniqueness, quantiles)",
                        )
                        .clicked()
                    {
                        self.run_describe();
                    }
                    if ui
                        .button("SELECT * LIMIT …")
                        .on_hover_text("Reset SQL for current data file (includes derived cols)")
                        .clicked()
                    {
                        if let Some(p) = &self.open_path {
                            if FolderTree::is_tabular(p) {
                                self.sql = derive::select_star_with_derived(
                                    p,
                                    &self.derived,
                                    self.preview_limit,
                                );
                            }
                        }
                    }
                    if ui.button("Clear").clicked() {
                        self.sql.clear();
                    }
                });
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .id_salt("sql_editor_scroll")
                    .auto_shrink([false, false])
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.sql)
                                .desired_width(f32::INFINITY)
                                .desired_rows(6)
                                .font(egui::TextStyle::Monospace)
                                .frame(true)
                                .hint_text("SELECT * FROM 'file.parquet' LIMIT 100"),
                        );
                    });
            });

        // ── Left: folder tree ────────────────────────────────────────────
        egui::SidePanel::left("tree_panel")
            .resizable(true)
            .default_width(280.0)
            .min_width(180.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_PANEL)
                    .inner_margin(egui::Margin::same(10))
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                theme::section_label(ui, "Files");
                ui.label(
                    egui::RichText::new(".parquet  .csv  .json  .jsonl  .sql")
                        .small()
                        .color(theme::TEXT_MUTED),
                );
                ui.label(
                    egui::RichText::new("Σ .sql → right-click → Load SQL")
                        .small()
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(4.0);
                let avail = ui.available_height();
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), avail),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if let Some(act) = self.tree.ui(ui) {
                            match act {
                                TreeAction::OpenData(p) => self.open_data_file(p),
                                TreeAction::LoadSql(p) => self.load_sql_file(p),
                            }
                        }
                    },
                );
            });

        // ── Right: schema ────────────────────────────────────────────────
        if self.show_schema {
            egui::SidePanel::right("schema_panel")
                .resizable(true)
                .default_width(240.0)
                .min_width(160.0)
                .frame(
                    egui::Frame::new()
                        .fill(theme::BG_PANEL)
                        .inner_margin(egui::Margin::same(10))
                        .stroke(egui::Stroke::new(1.0, theme::BORDER)),
                )
                .show(ctx, |ui| {
                    theme::section_label(ui, "Schema");
                    if let Some(path) = &self.open_path {
                        ui.label(
                            egui::RichText::new(
                                path.file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("file"),
                            )
                            .monospace()
                            .color(theme::TEXT),
                        );
                        if let Some(n) = self.total_rows {
                            ui.label(
                                egui::RichText::new(format!("{n} rows"))
                                    .small()
                                    .color(theme::TEXT_MUTED),
                            );
                        }
                    } else {
                        ui.weak("Select a data file");
                    }
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("schema_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if let Some(schema) = &self.schema {
                                for row in &schema.rows {
                                    if row.len() >= 2 {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&row[0])
                                                    .strong()
                                                    .color(theme::TEXT),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new(&row[1])
                                                            .small()
                                                            .monospace()
                                                            .color(theme::ACCENT_DIM),
                                                    );
                                                },
                                            );
                                        });
                                        ui.add_space(2.0);
                                    }
                                }
                            } else {
                                ui.weak("No schema");
                            }
                        });
                });
        }

        // ── Center: results (horizontal + vertical scroll) ───────────────
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    theme::section_label(ui, "Results");
                    if let Some(r) = &self.result {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} × {}",
                                r.columns.len(),
                                r.rows.len()
                            ))
                            .small()
                            .color(theme::TEXT_MUTED),
                        );
                    }
                    if ui
                        .small_button("Refresh")
                        .on_hover_text("Re-preview open file")
                        .clicked()
                    {
                        if let Some(p) = self.open_path.clone() {
                            if FolderTree::is_tabular(&p) {
                                self.open_data_file(p);
                            }
                        }
                    }
                    if ui
                        .small_button("Copy")
                        .on_hover_text("Copy results as TSV to clipboard")
                        .clicked()
                    {
                        self.copy_results(ctx);
                    }
                });
                ui.add_space(6.0);

                let Some(result) = self.result.as_ref() else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            egui::RichText::new("No results yet")
                                .size(16.0)
                                .color(theme::TEXT_MUTED),
                        );
                        ui.label(
                            egui::RichText::new(
                                "Pick a data file, or write SQL and Run · Describe table for profiles",
                            )
                            .small()
                            .color(theme::TEXT_MUTED),
                        );
                    });
                    return;
                };

                if result.columns.is_empty() {
                    ui.weak("(empty result set)");
                    return;
                }

                let text_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;
                let cols = result.columns.clone();
                let rows = result.rows.clone();
                let avail_h = ui.available_height().max(120.0);
                let col_w = 140.0_f32;
                let table_w = (cols.len() as f32) * col_w;

                // Independent horizontal scroll for wide results.
                egui::ScrollArea::horizontal()
                    .id_salt("results_hscroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(table_w.max(ui.available_width()));
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .columns(
                                Column::initial(col_w)
                                    .at_least(80.0)
                                    .clip(true)
                                    .resizable(true),
                                cols.len(),
                            )
                            .vscroll(true)
                            .min_scrolled_height(avail_h - 8.0)
                            .max_scroll_height(avail_h - 8.0)
                            .header(22.0, |mut header| {
                                for c in &cols {
                                    header.col(|ui| {
                                        ui.label(
                                            egui::RichText::new(c)
                                                .strong()
                                                .color(theme::ACCENT)
                                                .small(),
                                        );
                                    });
                                }
                            })
                            .body(|body| {
                                body.rows(text_height, rows.len(), |mut row| {
                                    let r = row.index();
                                    for cell in &rows[r] {
                                        row.col(|ui| {
                                            // Multi-line JSON pretty cells: show first line + …
                                            let display = if cell.contains('\n') {
                                                let first = cell.lines().next().unwrap_or("");
                                                if cell.lines().count() > 1 {
                                                    format!("{first} …")
                                                } else {
                                                    first.to_string()
                                                }
                                            } else if cell.chars().count() > 120 {
                                                let t: String = cell.chars().take(120).collect();
                                                format!("{t}…")
                                            } else if cell.is_empty() {
                                                "∅".into()
                                            } else {
                                                cell.clone()
                                            };
                                            let color = if cell.is_empty() {
                                                theme::TEXT_MUTED
                                            } else {
                                                theme::TEXT
                                            };
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(display)
                                                        .color(color)
                                                        .monospace(),
                                                )
                                                .truncate(),
                                            )
                                            .on_hover_text(cell);
                                        });
                                    }
                                });
                            });
                    });
            });
    }
}

fn find_first_tabular(dir: &std::path::Path, depth: u32) -> Option<PathBuf> {
    if depth > 6 {
        return None;
    }
    let rd = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_file() && FolderTree::is_supported(&p) && !FolderTree::is_sql(&p) {
            return Some(p);
        } else if p.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && name != "lost+found" && name != "build-cache" {
                subdirs.push(p);
            }
        }
    }
    for sub in subdirs {
        if let Some(found) = find_first_tabular(&sub, depth + 1) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_datalake_parquet_files() {
        let files = [
            "/mnt/datalake/kinnalake/nonprod/lake_us/lake/lz/kinnaruns/domain=timberland.com/report_date=2026-07-29/run_id=182459Z-3872/raw_snoop/crawlplan_siteinfo.parquet",
            "/mnt/datalake/kinnalake/nonprod/lake_us/lake/lz/kinnaruns/domain=timberland.com/report_date=2026-07-29/run_id=182459Z-3872/raw_snoop/crawlplan_products_raw.parquet",
            "/mnt/datalake/kinnalake/nonprod/lake_us/lake/lz/kinnaruns/domain=timberland.com/report_date=2026-07-29/run_id=182459Z-3872/raw_scrape/enriched_scrape.parquet",
            "/mnt/datalake/kinnalake/nonprod/lake_us/lake/lz/kinnaruns/domain=timberland.com/report_date=2026-07-29/run_id=182459Z-3872/raw_capture/raw_capture.parquet",
        ];
        let mut app = ParqApp::default();
        for f in files {
            let p = PathBuf::from(f);
            if !p.exists() {
                continue;
            }
            app.open_data_file(p.clone());
            assert!(app.error.is_none(), "error opening {f}: {:?}", app.error);
            assert!(app.result.is_some(), "no result for {f}");
            let r = app.result.as_ref().unwrap();
            assert!(!r.columns.is_empty(), "empty columns for {f}");
        }
    }
}
