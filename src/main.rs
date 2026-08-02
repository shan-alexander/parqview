//! parqview — lightweight egui explorer for Parquet/CSV/JSON/SQL via system DuckDB.
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

    // On Linux, default to WINIT_UNIX_BACKEND=x11 unless explicitly specified.
    // X11 / Xwayland provides universal display compatibility for standalone cargo binaries.
    if std::env::var_os("WINIT_UNIX_BACKEND").is_none() {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

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
