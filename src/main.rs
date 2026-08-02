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

    // If WINIT_UNIX_BACKEND is not set and WAYLAND_DISPLAY is missing, default to X11.
    if std::env::var("WINIT_UNIX_BACKEND").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
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

    let initial_clone = initial.clone();
    let res = eframe::run_native(
        "parqview",
        options.clone(),
        Box::new(move |cc| Ok(Box::new(app::ParqApp::new(cc, initial_clone)))),
    );

    if let Err(ref e) = res {
        if std::env::var("WINIT_UNIX_BACKEND").is_err() {
            eprintln!("Wayland initialization failed ({e}). Falling back to X11 backend...");
            std::env::set_var("WINIT_UNIX_BACKEND", "x11");
            return eframe::run_native(
                "parqview",
                options,
                Box::new(move |cc| Ok(Box::new(app::ParqApp::new(cc, initial)))),
            );
        }
    }

    res
}
