//! Thin oracle harness: drives legacy `HeadlessWrapper.lua` on pinned LuaJIT
//! at a pinned public PoB tag and emits normalized results, recording all
//! run metadata (ADR-006). Legacy source is fetched, never copied in.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("loom-parity-cli: not implemented yet (Spike C, docs/plan/ACTION-PLAN.md)");
    ExitCode::FAILURE
}
