//! parqview — lightweight egui explorer for Parquet/CSV/JSON/SQL via system DuckDB.
//!
//! Prefers Wayland when available (no forced X11).
//!
//! Usage:
//!   parqview
//!   parqview /path/to/file.parquet
//!   parqview /path/to/folder

mod app;
mod derive;
mod describe;
mod duck;
mod theme;
mod tree;

use std::path::PathBuf;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let initial = std::env::args().nth(1).map(PathBuf::from);

    // Wayland-first: do not set WINIT_UNIX_BACKEND=x11.
    // OpenGL (glow) is more portable than wgpu on NixOS GPU stacks.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("parqview"),
        renderer: eframe::Renderer::Glow,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "parqview",
        options,
        Box::new(move |cc| Ok(Box::new(app::ParqApp::new(cc, initial)))),
    )
}
