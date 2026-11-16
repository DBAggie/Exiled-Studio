//! `loom-bench`: the core paths the performance budgets cover (Phase 5,
//! P7; plan decision P-h; `docs/phase-5/performance.md`).
//!
//! Each scenario's work is one `measured_*` function, so callgrind counts
//! only that work (`tools/bench/count.sh` toggles collection on those
//! functions) and `wall` times only that work. The inputs are read first.
//!
//! | Scenario | Work | Budget it feeds |
//! |---|---|---|
//! | `validate` | the bundle validator with every file's hash, as a folder chosen by hand is checked | startup (development folders) |
//! | `load` | the bundle's tables, the resolver, the passive tree | startup |
//! | `start` | what the start does with an installed bundle: the validator with sizes only, which loads the tables once, then the resolver and the passive tree | startup |
//! | `import` | a PoB build read and resolved against the bundle | import and first calculation |
//! | `calculate` | one calculation of the resolved build | import and first calculation; item-edit and allocation previews |
//! | `allocate` | the path to a node on the tree (an allocation preview's tree part) | allocation preview |

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use loom_data::bundle::{Bundle, MemorySource};
use loom_parser::import::{BundleData, Resolver};
use loom_tree::Tree;
use loom_tree::poe1::{self as rules, Spec};

const USAGE: &str = "\
usage:
  loom-bench run <validate|load|start|import|calculate|allocate> [--bundle <dir>] [--build <file>]
      one measured run (for callgrind; tools/bench/count.sh)
  loom-bench wall [--bundle <dir>] [--build <file>] [--runs <n>]
      each scenario's median and slowest wall-clock time over <n> runs (default 20)

The defaults are the synthetic tree bundle (crates/loom-data/tests/fixtures/
bundle-poe1-tree) and the largest public PoB fixture.";

const SCENARIOS: [&str; 6] = [
    "validate",
    "load",
    "start",
    "import",
    "calculate",
    "allocate",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn run(args: &[String]) -> Result<(), String> {
    let bundle = flag(args, "--bundle").map_or_else(
        || root().join("crates/loom-data/tests/fixtures/bundle-poe1-tree"),
        PathBuf::from,
    );
    let build = flag(args, "--build").map_or_else(
        || {
            root()
                .join("tools/oracle/fixtures/community/forged-frostbearer-spectre-necromancer.xml")
        },
        PathBuf::from,
    );
    match args.first().map(String::as_str) {
        Some("run") => {
            let scenario = args.get(1).ok_or(USAGE)?;
            let inputs = Inputs::read(&bundle, &build)?;
            let loaded = Loaded::new(&inputs)?;
            once(scenario, &inputs, &loaded)
        }
        Some("wall") => {
            let runs: usize = flag(args, "--runs")
                .unwrap_or("20")
                .parse()
                .map_err(|_| "--runs: a whole number".to_owned())?;
            let inputs = Inputs::read(&bundle, &build)?;
            let loaded = Loaded::new(&inputs)?;
            println!("bundle: {}", bundle.display());
            println!("build: {}", build.display());
            for scenario in SCENARIOS {
                let mut times = Vec::with_capacity(runs.max(1));
                for _ in 0..runs.max(1) {
                    let start = Instant::now();
                    once(scenario, &inputs, &loaded)?;
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                }
                times.sort_by(f64::total_cmp);
                let median = times.get(times.len() / 2).copied().unwrap_or_default();
                let slowest = times.last().copied().unwrap_or_default();
                println!(
                    "{scenario}: median {median:.1} ms, slowest {slowest:.1} ms ({} runs)",
                    times.len()
                );
            }
            Ok(())
        }
        _ => Err(USAGE.to_owned()),
    }
}

/// The files, read before anything is measured.
struct Inputs {
    source: MemorySource,
    pob: String,
}

impl Inputs {
    fn read(bundle: &Path, build: &Path) -> Result<Self, String> {
        let mut source = MemorySource::new();
        let mut stack = vec![bundle.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            for entry in entries {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(bundle) {
                    let bytes =
                        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                    source.insert(rel.to_string_lossy().replace('\\', "/"), bytes);
                }
            }
        }
        let pob =
            std::fs::read_to_string(build).map_err(|e| format!("{}: {e}", build.display()))?;
        Ok(Self { source, pob })
    }
}

/// What the later scenarios start from: the loaded data, the resolved
/// build, and a node to allocate.
struct Loaded {
    bundle: &'static Bundle,
    data: &'static BundleData<'static>,
    resolver: &'static Resolver<'static, 'static>,
    tree: Tree,
    build: loom_domain::Build,
    ends: loom_pipeline::RangeEnds,
    spec: Spec,
    target: u32,
}

impl Loaded {
    fn new(inputs: &Inputs) -> Result<Self, String> {
        let bundle: &'static Bundle = Box::leak(Box::new(
            Bundle::load(&inputs.source).map_err(|e| e.to_string())?,
        ));
        let data: &'static BundleData<'static> = Box::leak(Box::new(BundleData::new(bundle)));
        let resolver: &'static Resolver<'static, 'static> =
            Box::leak(Box::new(Resolver::new(data).map_err(|e| e.to_string())?));
        let tree = Tree::from_bundle(bundle).ok_or("the bundle has no passive tree")?;
        let mut build = loom_pipeline::read_pob(&inputs.pob).map_err(|e| e.to_string())?;
        let ends = loom_pipeline::resolve(resolver, data, &mut build).map_err(|e| e.to_string())?;
        let active =
            loom_calc::poe1::collect::active_spec(&build).ok_or("the build has no spec")?;
        let spec = loom_app::tree::rules_spec(&build, active, &tree);
        // The first node (by ID) whose path is at least five long: a
        // preview that searches, not a neighbour's one step.
        let target = tree
            .nodes
            .keys()
            .copied()
            .find(|&n| rules::allocate(&tree, &spec, n).is_ok_and(|a| a.path.len() >= 5))
            .ok_or("no node to allocate")?;
        Ok(Self {
            bundle,
            data,
            resolver,
            tree,
            build,
            ends,
            spec,
            target,
        })
    }
}

fn once(scenario: &str, inputs: &Inputs, loaded: &Loaded) -> Result<(), String> {
    match scenario {
        "validate" => measured_validate(&inputs.source),
        "load" => measured_load(&inputs.source),
        "start" => measured_start(&inputs.source),
        "import" => measured_import(&inputs.pob, loaded),
        "calculate" => measured_calculate(loaded),
        "allocate" => measured_allocate(loaded),
        _ => Err(USAGE.to_owned()),
    }
}

#[inline(never)]
fn measured_validate(source: &MemorySource) -> Result<(), String> {
    let report = loom_data::bundle::validate::validate(source);
    if !report.is_valid() {
        return Err(format!(
            "not a valid bundle ({})",
            report.codes().join(", ")
        ));
    }
    Ok(())
}

#[inline(never)]
fn measured_load(source: &MemorySource) -> Result<(), String> {
    let bundle = Bundle::load(source).map_err(|e| e.to_string())?;
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).map_err(|e| e.to_string())?;
    black_box(&resolver);
    black_box(Tree::from_bundle(&bundle));
    Ok(())
}

#[inline(never)]
fn measured_start(source: &MemorySource) -> Result<(), String> {
    use loom_data::bundle::validate::{Integrity, check};
    let bundle = check(source, Integrity::Sizes)
        .map_err(|report| format!("not a valid bundle ({})", report.codes().join(", ")))?;
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).map_err(|e| e.to_string())?;
    black_box(&resolver);
    black_box(Tree::from_bundle(&bundle));
    Ok(())
}

#[inline(never)]
fn measured_import(pob: &str, loaded: &Loaded) -> Result<(), String> {
    let mut build = loom_pipeline::read_pob(pob).map_err(|e| e.to_string())?;
    let ends = loom_pipeline::resolve(loaded.resolver, loaded.data, &mut build)
        .map_err(|e| e.to_string())?;
    black_box((build, ends));
    Ok(())
}

#[inline(never)]
fn measured_calculate(loaded: &Loaded) -> Result<(), String> {
    let options = loom_pipeline::options(
        &loaded.build,
        loaded.data,
        &loaded.ends,
        None,
        Some(&loaded.tree),
    );
    let result = loom_pipeline::calculate(&loaded.build, loaded.bundle, &options, &|| false)
        .map_err(|e| e.to_string())?;
    black_box(result);
    Ok(())
}

#[inline(never)]
fn measured_allocate(loaded: &Loaded) -> Result<(), String> {
    let allocation =
        rules::allocate(&loaded.tree, &loaded.spec, loaded.target).map_err(|e| e.to_string())?;
    black_box(allocation);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SCENARIOS;

    /// CI counts the scenarios `count.sh` names; one left out of it is
    /// never counted, so it could never fail.
    #[test]
    fn count_sh_should_count_every_scenario_in_order() {
        let script = include_str!("../count.sh");
        let line = script
            .lines()
            .find_map(|l| l.strip_prefix("for scenario in "))
            .and_then(|l| l.strip_suffix("; do"))
            .expect("count.sh has the scenario loop");
        let listed: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(listed, SCENARIOS);
    }
}
