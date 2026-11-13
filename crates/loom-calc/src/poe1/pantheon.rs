//! The pantheon (P3-12g): the souls of the chosen major and minor gods.
//!
//! **PoB v2.67.2.**
//! - `calcs.initModDB` (`Modules/CalcSetup.lua` 560 to 571) reads
//!   `env.configInput.pantheonMajorGod` and `pantheonMinorGod`; for a
//!   choice other than "None" it applies `pantheon.applySoulMod`
//!   (`Modules/PantheonTools.lua`): every soul of the god, each mod's line
//!   through the mod parser.
//! - Every soul counts, upgrades included: PoB has no input for which
//!   upgrades are unlocked, so a chosen god is a fully upgraded one.
//! - The gods are `data.pantheons` (`Data/Pantheons.lua`), exported from
//!   the game's `PantheonPanelLayout` (`Export/Scripts/pantheons.lua`):
//!   each soul's mod is a stat with its value, written as the line the
//!   game's stat description gives it.
//! - Which value PoB reads is the bandit's rule (`collect::choices`): the
//!   active config set's input, "None" when the set has none; the build
//!   element's attribute only for a build without a config element.
//!
//! **The engine.** The bundle carries the same table
//! (`poe1/pantheons.json`; `data-parity` compares it with PoB's, line by
//! line). A chosen god's soul stats act through the stat mapping, as an
//! item's would, with the config input as their source: a stat the mapping
//! does not know is a reason (`unmapped-stat`), never dropped. A name the
//! table does not hold, or a value that is not a name, is a reason under
//! the input (PoB would fail on it); data without the table is
//! `Indeterminate` (`missing-data`, table `pantheons`).
//!
//! PoB drops a line its parser does not read in full (`if modList and not
//! extra`); the engine reads the stat instead. No soul stat reaches a
//! slice field (each is a `NoEffect` row or unmapped), so the two agree on
//! every field (a test lists them).

use loom_data::bundle::Bundle;
use loom_data::bundle::poe1::Pantheon;
use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue, StatId, StatLine, StatValue};

use crate::ingest::Ingest;
use crate::input::CalcData;
use crate::source::Source;
use crate::state::{IndeterminateReason, UnsupportedReason};

/// The Path of Exile only tables a calculation reads
/// ([`CalcData::poe1`]).
pub trait Poe1Data {
    /// The pantheon's gods; `None` when the data has no such table.
    fn pantheons(&self) -> Option<&[Pantheon]>;
}

impl Poe1Data for Bundle {
    fn pantheons(&self) -> Option<&[Pantheon]> {
        self.poe1.as_ref()?.pantheons.as_deref()
    }
}

/// The config input of the major god.
pub const MAJOR_INPUT: &str = "pantheonMajorGod";

/// The config input of the minor god.
pub const MINOR_INPUT: &str = "pantheonMinorGod";

/// PoB's value for no god.
pub const NONE: &str = "None";

/// Whether `name` is one of the two inputs (their effects are here, not
/// config reasons).
#[must_use]
pub fn is_input(name: &str) -> bool {
    name == MAJOR_INPUT || name == MINOR_INPUT
}

/// The active set's gods: each input's last value acts ("None", or no
/// input, acts as no god).
pub fn pantheon(set: &ConfigSet, data: &dyn CalcData, ingest: &mut Ingest<'_>) {
    for input in [MAJOR_INPUT, MINOR_INPUT] {
        let name = match super::config::last_input(set, input, ConfigInputKind::Input) {
            None => continue,
            Some(ConfigValue::String(s)) => s.as_str(),
            Some(_) => {
                ingest.reason(UnsupportedReason::UnmodelledConfig {
                    input: input.to_owned(),
                });
                continue;
            }
        };
        if name == NONE {
            continue;
        }
        apply(input, name, data, ingest);
    }
}

/// Applies the god `name` chosen in `input`.
fn apply(input: &str, name: &str, data: &dyn CalcData, ingest: &mut Ingest<'_>) {
    let Some(gods) = data.poe1().and_then(Poe1Data::pantheons) else {
        ingest.indeterminate(IndeterminateReason::MissingData {
            table: "pantheons",
            key: name.to_owned(),
        });
        return;
    };
    let Some(god) = gods.iter().find(|g| g.id == name) else {
        ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: input.to_owned(),
        });
        return;
    };
    let source = Source::Config {
        input: input.to_owned(),
    };
    for (stat, value) in god.souls.iter().flat_map(|s| &s.stats) {
        ingest.stat(
            &source,
            &StatLine {
                stat: StatId::new(stat.as_str()),
                value: StatValue::Exact {
                    min: *value,
                    max: *value,
                },
            },
        );
    }
}
