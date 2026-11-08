# ADR-002: Application Shell and Domain Core

- Status: Accepted
- Date: 2026-02-01

## Decision
Use Tauri 2 as the desktop shell, React with TypeScript and Vite for the UI, and an in-process Rust core. Use a Cargo workspace with independent crates for domain, calculation, data, parsing, serialization, application orchestration, persistence, integrations, and the Tauri adapter.

The web UI is presentation only. The Rust core has no Tauri or UI dependencies. Tauri commands, queries, and events use generated Rust-to-TypeScript bindings.

## Consequences
- Smaller desktop footprint than Electron is targeted.
- Windows WebView2 and Linux WebKitGTK require cross-platform UI testing.
- The core can later support headless or service use without requiring IPC.
- The workspace must maintain one-way dependency flow and avoid circular contracts.
