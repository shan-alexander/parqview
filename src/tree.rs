//! Folder tree browser — only surfaces .parquet / .csv / .json / .jsonl / .sql (and dirs).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;

use crate::theme;

const SUPPORTED_EXT: &[&str] = &["parquet", "csv", "tsv", "json", "jsonl", "ndjson", "sql"];

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// Result of interacting with the tree.
#[derive(Debug, Clone)]
pub enum TreeAction {
    /// Open tabular data (left-click on data file).
    OpenData(PathBuf),
    /// Load SQL into editor (context menu only).
    LoadSql(PathBuf),
}

#[derive(Default)]
pub struct FolderTree {
    pub root: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    cache: HashMap<PathBuf, Vec<DirEntry>>,
    pub selected: Option<PathBuf>,
    pub error: Option<String>,
}

pub fn path_breadcrumbs(path: &Path) -> Vec<(String, PathBuf)> {
    let mut crumbs = Vec::new();
    let mut current = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::RootDir => {
                current.push(std::path::MAIN_SEPARATOR.to_string());
                crumbs.push(("/".to_string(), current.clone()));
            }
            std::path::Component::Prefix(prefix) => {
                let s = prefix.as_os_str().to_string_lossy().to_string();
                current.push(&s);
                crumbs.push((s, current.clone()));
            }
            std::path::Component::Normal(name) => {
                let s = name.to_string_lossy().to_string();
                current.push(&s);
                crumbs.push((s, current.clone()));
            }
            _ => {}
        }
    }
    crumbs
}

impl FolderTree {
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root.clone());
        self.expanded.clear();
        self.cache.clear();
        self.expanded.insert(root);
        self.error = None;
        self.selected = None;
    }

    pub fn refresh(&mut self) {
        self.cache.clear();
        self.error = None;
    }

    pub fn is_supported(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| SUPPORTED_EXT.iter().any(|s| e.eq_ignore_ascii_case(s)))
            .unwrap_or(false)
    }

    pub fn is_sql(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("sql"))
            .unwrap_or(false)
    }

    pub fn is_tabular(path: &Path) -> bool {
        if path.is_dir() {
            return Self::dir_has_tabular(path);
        }
        Self::is_supported(path) && !Self::is_sql(path)
    }

    pub fn dir_has_tabular(dir: &Path) -> bool {
        if let Ok(rd) = fs::read_dir(dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.is_file() && (Self::is_supported(&p) && !Self::is_sql(&p)) {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_jsonl(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                let e = e.to_ascii_lowercase();
                e == "jsonl" || e == "ndjson"
            })
            .unwrap_or(false)
    }

    fn list_dir(&mut self, dir: &Path) -> &[DirEntry] {
        if !self.cache.contains_key(dir) {
            let mut entries = Vec::new();
            match fs::read_dir(dir) {
                Ok(rd) => {
                    for ent in rd.flatten() {
                        let path = ent.path();
                        let name = ent.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') {
                            continue;
                        }
                        let is_dir = path.is_dir();
                        if is_dir {
                            entries.push(DirEntry {
                                name,
                                path,
                                is_dir: true,
                            });
                        } else if Self::is_supported(&path) {
                            entries.push(DirEntry {
                                name,
                                path,
                                is_dir: false,
                            });
                        }
                    }
                    entries.sort_by(|a, b| {
                        b.is_dir
                            .cmp(&a.is_dir)
                            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                    });
                }
                Err(e) => {
                    self.error = Some(format!("read {}: {e}", dir.display()));
                }
            }
            self.cache.insert(dir.to_path_buf(), entries);
        }
        self.cache.get(dir).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Draw tree; returns open/load actions.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<TreeAction> {
        let mut action = None;
        let root = match &self.root {
            Some(r) => r.clone(),
            None => {
                ui.weak("No folder open");
                ui.add_space(6.0);
                if ui.button("Open folder…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        self.set_root(p);
                    }
                }
                return None;
            }
        };

        let mut new_root = None;

        ui.horizontal(|ui| {
            if let Some(parent) = root.parent() {
                if ui
                    .button("⬆ Up")
                    .on_hover_text(format!("Navigate upstream to: {}", parent.display()))
                    .clicked()
                {
                    new_root = Some(parent.to_path_buf());
                }
            }
            if ui
                .small_button("↻")
                .on_hover_text("Refresh folder tree")
                .clicked()
            {
                self.refresh();
            }
            if ui
                .small_button("🏠 CWD")
                .on_hover_text("Reset root to current working directory")
                .clicked()
            {
                if let Ok(cwd) = std::env::current_dir() {
                    new_root = Some(cwd);
                }
            }
            if ui
                .small_button("📁 Browse…")
                .on_hover_text("Choose a folder via OS dialog")
                .clicked()
            {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    new_root = Some(p);
                }
            }
        });

        ui.add_space(2.0);

        let crumbs = path_breadcrumbs(&root);
        ui.horizontal_wrapped(|ui| {
            let last_idx = crumbs.len().saturating_sub(1);
            for (idx, (label, target_path)) in crumbs.into_iter().enumerate() {
                let is_last = idx == last_idx;
                let text = if is_last {
                    egui::RichText::new(&label).strong().color(theme::ACCENT)
                } else {
                    egui::RichText::new(&label).color(theme::TEXT_MUTED)
                };
                if ui
                    .link(text)
                    .on_hover_text(format!("Navigate upstream to {}", target_path.display()))
                    .clicked()
                {
                    new_root = Some(target_path);
                }
                if !is_last && label != "/" {
                    ui.label(egui::RichText::new("/").small().color(theme::TEXT_MUTED));
                }
            }
        });

        if let Some(nr) = new_root {
            self.set_root(nr);
            return None;
        }

        ui.add_space(4.0);
        ui.separator();

        egui::ScrollArea::both()
            .id_salt("folder_tree_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                action = self.draw_dir(ui, &root, 0);
            });

        if let Some(err) = &self.error {
            ui.colored_label(theme::ERR, err);
        }
        action
    }

    fn draw_dir(&mut self, ui: &mut egui::Ui, dir: &Path, depth: u32) -> Option<TreeAction> {
        let mut action = None;
        let entries: Vec<DirEntry> = self.list_dir(dir).to_vec();
        let total_entries = entries.len();
        let max_render = 300;
        let (render_entries, truncated) = if total_entries > max_render {
            (&entries[..max_render], total_entries - max_render)
        } else {
            (&entries[..], 0)
        };

        for ent in render_entries {
            let indent = (depth as f32) * 14.0;
            if ent.is_dir {
                let is_open = self.expanded.contains(&ent.path);
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    let icon = if is_open { "▾" } else { "▸" };
                    let label = format!("{icon} 📁 {}", ent.name);
                    let resp =
                        ui.selectable_label(false, egui::RichText::new(label).color(theme::TEXT));
                    if resp.clicked() {
                        if is_open {
                            self.expanded.remove(&ent.path);
                        } else {
                            self.expanded.insert(ent.path.clone());
                        }
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Open folder as dataset").clicked() {
                            self.selected = Some(ent.path.clone());
                            action = Some(TreeAction::OpenData(ent.path.clone()));
                            ui.close_menu();
                        }
                    });
                });
                if self.expanded.contains(&ent.path) {
                    if let Some(a) = self.draw_dir(ui, &ent.path, depth + 1) {
                        action = Some(a);
                    }
                }
            } else {
                let selected = self.selected.as_ref() == Some(&ent.path);
                let is_sql = Self::is_sql(&ent.path);
                let icon = file_icon(&ent.path);
                ui.horizontal(|ui| {
                    ui.add_space(indent + 4.0);
                    let color = if selected {
                        theme::ACCENT
                    } else if is_sql {
                        theme::TEXT_MUTED
                    } else {
                        theme::TEXT
                    };
                    let text = egui::RichText::new(format!("{icon} {}", ent.name)).color(color);
                    let resp = ui.selectable_label(selected, text);

                    // SQL: left-click does nothing useful; right-click menu to load.
                    if is_sql {
                        resp.context_menu(|ui| {
                            if ui.button("Load SQL into editor").clicked() {
                                self.selected = Some(ent.path.clone());
                                action = Some(TreeAction::LoadSql(ent.path.clone()));
                                ui.close_menu();
                            }
                        });
                        // Optional: select highlight without loading
                        if resp.clicked() {
                            self.selected = Some(ent.path.clone());
                        }
                    } else {
                        if resp.clicked() || resp.double_clicked() {
                            self.selected = Some(ent.path.clone());
                            action = Some(TreeAction::OpenData(ent.path.clone()));
                        }
                        resp.context_menu(|ui| {
                            if ui.button("Open").clicked() {
                                self.selected = Some(ent.path.clone());
                                action = Some(TreeAction::OpenData(ent.path.clone()));
                                ui.close_menu();
                            }
                        });
                    }
                });
            }
        }
        if truncated > 0 {
            let indent = (depth as f32) * 14.0;
            ui.horizontal(|ui| {
                ui.add_space(indent + 4.0);
                ui.label(
                    egui::RichText::new(format!("… and {truncated} more entries"))
                        .small()
                        .italics()
                        .color(theme::TEXT_MUTED),
                );
            });
        }
        action
    }
}

fn file_icon(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "parquet" => "▣",
        "csv" | "tsv" => "▦",
        "json" => "{ }",
        "jsonl" | "ndjson" => "☰",
        "sql" => "Σ",
        _ => "·",
    }
}
