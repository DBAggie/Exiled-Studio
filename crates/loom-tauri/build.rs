//! Generates the Tauri context (config, capabilities, Windows resources),
//! and on Windows with MSVC embeds the Common Controls v6 manifest in the
//! test binaries (`windows-test.manifest`), which Tauri needs to start.
//!
//! `LOOM_CHANNEL` names the release channel (`docs/release/channels.md`):
//! unset or `stable`, or `experimental`, which sets `cfg(loom_experimental)`
//! (`loom_tauri::release::Channel`). Any other value fails the build.

fn main() {
    println!("cargo:rustc-check-cfg=cfg(loom_experimental)");
    println!("cargo:rerun-if-env-changed=LOOM_CHANNEL");
    match std::env::var("LOOM_CHANNEL").as_deref() {
        Ok("experimental") => println!("cargo:rustc-cfg=loom_experimental"),
        Ok("" | "stable") | Err(_) => {}
        Ok(other) => panic!("LOOM_CHANNEL is {other:?}: stable or experimental"),
    }
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        let manifest =
            std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
                .join("windows-test.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
    tauri_build::build();
}
