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

fn is_wayland_available() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return false;
    }
    #[cfg(target_os = "linux")]
    unsafe {
        if let Ok(name1) = std::ffi::CString::new("libwayland-client.so.0") {
            let handle1 = libc::dlopen(name1.as_ptr(), libc::RTLD_LAZY);
            if !handle1.is_null() {
                libc::dlclose(handle1);
                return true;
            }
        }
        if let Ok(name2) = std::ffi::CString::new("libwayland-client.so") {
            let handle2 = libc::dlopen(name2.as_ptr(), libc::RTLD_LAZY);
            if !handle2.is_null() {
                libc::dlclose(handle2);
                return true;
            }
        }
        false
    }
    #[cfg(not(target_os = "linux"))]
    true
}

fn main() -> eframe::Result<()> {
    let initial = std::env::args().nth(1).map(PathBuf::from);

    // If WINIT_UNIX_BACKEND is not set, check if Wayland library & display are available.
    if std::env::var_os("WINIT_UNIX_BACKEND").is_none() && !is_wayland_available() {
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
