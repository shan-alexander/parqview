//! parqview — lightweight egui explorer for Parquet/CSV/JSON/SQL via system DuckDB.
//!
//! Usage:
//!   parqview
//!   parqview /path/to/file.parquet
//!   parqview /path/to/folder
//!
//! Display (Linux):
//!   Prefer Wayland when the compositor socket and shared libraries are available.
//!   Set `PARQVIEW_X11=1` to force X11 / XWayland.
//!   On NixOS, cargo-installed binaries re-exec with nix-ld GUI library paths when needed.

mod app;
mod derive;
mod describe;
mod duck;
mod theme;
mod tree;

use std::path::PathBuf;

use eframe::egui;

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "linux")]
    linux::prepare_display_backend();

    let initial = std::env::args().nth(1).map(PathBuf::from);

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

/// Linux display/backend setup for winit 0.30+ (eframe).
///
/// winit no longer honors `WINIT_UNIX_BACKEND`. Backend choice is:
/// - Wayland if `WAYLAND_DISPLAY` / `WAYLAND_SOCKET` is set
/// - else X11 if `DISPLAY` is set
///
/// On NixOS, plain `cargo install` binaries cannot see Wayland/X11/GL libs unless
/// `LD_LIBRARY_PATH` includes them. Setting that env var after process start does
/// **not** affect `dlopen`, so we re-exec once with the right path.
#[cfg(target_os = "linux")]
mod linux {
    use std::ffi::CString;
    use std::os::unix::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const REEXEC_ENV: &str = "PARQVIEW_LIB_REEXEC";

    pub fn prepare_display_backend() {
        ensure_gui_library_path();

        let force_x11 = env_flag("PARQVIEW_X11") || !wayland_stack_available();
        if force_x11 {
            // winit 0.29+: clear Wayland env so it selects X11 when DISPLAY is set.
            remove_env("WAYLAND_DISPLAY");
            remove_env("WAYLAND_SOCKET");
        }
    }

    fn ensure_gui_library_path() {
        if wayland_stack_available() || x11_stack_available() {
            return;
        }
        if std::env::var_os(REEXEC_ENV).is_some() {
            return;
        }

        let mut dirs: Vec<PathBuf> = Vec::new();
        push_dir_if_exists(&mut dirs, Path::new("/run/current-system/sw/share/nix-ld/lib"));
        push_dir_if_exists(&mut dirs, Path::new("/run/opengl-driver/lib"));
        if let Some(home) = std::env::var_os("HOME") {
            push_dir_if_exists(&mut dirs, &PathBuf::from(home).join(".nix-profile/lib"));
        }
        // Guix / other FHS-ish layouts sometimes expose libs here.
        push_dir_if_exists(&mut dirs, Path::new("/usr/lib"));
        push_dir_if_exists(&mut dirs, Path::new("/usr/lib64"));
        push_dir_if_exists(&mut dirs, Path::new("/usr/lib/x86_64-linux-gnu"));

        // Only re-exec if at least one candidate actually contains a GUI lib we need.
        let useful: Vec<PathBuf> = dirs
            .into_iter()
            .filter(|d| {
                d.join("libwayland-client.so.0").exists()
                    || d.join("libwayland-client.so").exists()
                    || d.join("libX11.so.6").exists()
                    || d.join("libX11.so").exists()
            })
            .collect();

        if useful.is_empty() {
            return;
        }

        let extra = useful
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(":");

        let new_ld = match std::env::var("LD_LIBRARY_PATH") {
            Ok(existing) if !existing.is_empty() => format!("{extra}:{existing}"),
            _ => extra,
        };

        let Ok(exe) = std::env::current_exe() else {
            return;
        };

        let mut cmd = Command::new(exe);
        cmd.args(std::env::args().skip(1));
        cmd.env("LD_LIBRARY_PATH", &new_ld);
        cmd.env(REEXEC_ENV, "1");

        let err = cmd.exec();
        eprintln!(
            "parqview: failed to re-exec with GUI library path ({new_ld}): {err}\n\
             Hint: on NixOS use `nix run .` / `./run.sh`, or set LD_LIBRARY_PATH to Wayland/X11/GL libs."
        );
    }

    fn wayland_stack_available() -> bool {
        if !env_nonempty("WAYLAND_DISPLAY") && !env_nonempty("WAYLAND_SOCKET") {
            return false;
        }
        // winit/glutin need these for a working Wayland + EGL path.
        can_dlopen(&[
            "libwayland-client.so.0",
            "libxkbcommon.so.0",
            "libwayland-egl.so.1",
            "libEGL.so.1",
        ])
    }

    fn x11_stack_available() -> bool {
        if !env_nonempty("DISPLAY") {
            return false;
        }
        can_dlopen(&["libX11.so.6"])
    }

    fn can_dlopen(libs: &[&str]) -> bool {
        for lib in libs {
            if !dlopen_ok(lib) {
                return false;
            }
        }
        true
    }

    fn dlopen_ok(lib: &str) -> bool {
        let Ok(cname) = CString::new(lib) else {
            return false;
        };
        // SAFETY: dlopen/dlclose with a valid C string; we only probe and close immediately.
        unsafe {
            let handle = libc::dlopen(cname.as_ptr(), libc::RTLD_LAZY);
            if handle.is_null() {
                return false;
            }
            libc::dlclose(handle);
            true
        }
    }

    fn push_dir_if_exists(dirs: &mut Vec<PathBuf>, path: &Path) {
        if path.is_dir() {
            dirs.push(path.to_path_buf());
        }
    }

    fn env_nonempty(key: &str) -> bool {
        std::env::var_os(key).is_some_and(|v| !v.is_empty())
    }

    fn env_flag(key: &str) -> bool {
        matches!(
            std::env::var(key).as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
        )
    }

    fn remove_env(key: &str) {
        // SAFETY: called only from main before other threads are spawned.
        unsafe { std::env::remove_var(key) };
    }
}
