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
        // winit's Wayland backend requires libwayland-client, libxkbcommon, and libwayland-egl / libEGL.
        // If any of these are missing from LD_LIBRARY_PATH (e.g. NixOS outside nix develop), winit fails with NoWaylandLib.
        let required_libs = [
            "libwayland-client.so.0",
            "libxkbcommon.so.0",
            "libwayland-egl.so.1",
            "libEGL.so.1",
        ];
        for lib in required_libs {
            if let Ok(cname) = std::ffi::CString::new(lib) {
                let handle = libc::dlopen(cname.as_ptr(), libc::RTLD_LAZY);
                if handle.is_null() {
                    return false;
                }
                libc::dlclose(handle);
            } else {
                return false;
            }
        }
        true
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
