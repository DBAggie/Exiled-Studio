//! Desktop shell (ADR-002). Thin adapter only: domain logic stays in the core
//! crates and `loom-app`; the UI talks to Rust through generated bindings.

// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> tauri::Result<()> {
    tauri::Builder::default().run(tauri::generate_context!())
}
