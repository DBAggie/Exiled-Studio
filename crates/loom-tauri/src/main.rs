//! Desktop shell entry point (ADR-002): everything is in the library.

// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> tauri::Result<()> {
    loom_tauri::run()
}
