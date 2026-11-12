//! The enemy (Phase 3 plan, step 11): its level, resistances, armour, and
//! evasion as PoB v2.67.2's config describes it (P3-11), from the bosses
//! PoB averages for its boss enemies' armour and evasion (P3-1d). The hit
//! phases read it (P3-9b, P3-10).
//!
//! PoB's default enemy is a Guardian or Pinnacle boss (`enemyIsBoss`,
//! default index 3): its armour and evasion are the monster level's,
//! scaled by `100 + mean` percent over the multipliers of the bosses
//! [`BOSSES`] lists (`data.bossStats`, `src/Modules/Data.lua`); the Uber
//! boss uses the bosses marked uber. The multipliers are game data
//! (`MonsterTypes.Armour`, `.Evasion`, carried in the bundle's
//! `monster_types`); which bosses count is PoB's choice
//! (`src/Export/Enemies/Bosses.txt`), kept here as a rule with its source
//! (maintainer decision Q10, 2026-11-12, option a).
//!
//! Not modelled yet, for the hit phases (P3-9b, P3-10): the boss kind's
//! other modifiers (`Modules/ConfigOptions.lua`, `enemyIsBoss`): the
//! player's `WarcryPower` and `Multiplier:EnemyPower` +20, the enemy's
//! `Condition:RareOrUnique`, `Condition:PinnacleBoss`, `AilmentThreshold`
//! MORE, and the Uber boss's `DamageTaken` MORE -70, which changes hit
//! damage. So an `enemyIsBoss` input stays `unmodelled-config`, and the
//! default kind's modifiers must be modelled before a hit field is
//! `Complete`.

use loom_domain::{Build, ConfigValue};

use crate::input::CalcData;
use crate::math;

/// One boss PoB averages: its name in PoB, its monster type ID, and
/// whether it counts for the Uber boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Boss {
    /// PoB's name (`bosses["<name>"]`).
    pub name: &'static str,
    /// The monster type (`MonsterTypes.Id`).
    pub monster_type: &'static str,
    /// Whether the Uber boss counts it (`{Uber}`).
    pub uber: bool,
}

const fn boss(name: &'static str, monster_type: &'static str, uber: bool) -> Boss {
    Boss {
        name,
        monster_type,
        uber,
    }
}

/// PoB v2.67.2's boss list, in its file's order
/// (`src/Export/Enemies/Bosses.txt`, `#boss <name> <MonsterTypes.Id>
/// [{Uber}]`).
pub const BOSSES: [Boss; 22] = [
    boss("Venarius", "SynthesisVenarius", true),
    boss("EaterOfWorlds", "AtlasInvadersConsumeBoss", true),
    boss("SearingExarch", "AtlasInvadersCleansingBoss", true),
    boss("Maven", "TheMaven", true),
    boss("Sirus", "AtlasExiles5", true),
    boss("Shaper", "TheShaperBoss", true),
    boss("Elder", "TheElder", true),
    boss("BlackStar", "AtlasInvadersBlackStarBoss", false),
    boss("InfiniteHunger", "AtlasInvadersDoomBoss", false),
    boss("Atziri", "Atziri", false),
    boss("Phoenix", "AtlasBossPhoenix", false),
    boss("Hydra", "AtlasBossHydra", false),
    boss("Minotaur", "AtlasBossMinotaur", false),
    boss("Chimera", "AtlasBossChimera", false),
    boss("Enslaver", "ElderGuardian1", false),
    boss("Eradicator", "ElderGuardian2", false),
    boss("Constrictor", "ElderGuardian3", false),
    boss("Purifier", "ElderGuardian4", false),
    boss("Baran", "AtlasExiles1", false),
    boss("Veritania", "AtlasExiles2", false),
    boss("AlHezmin", "AtlasExiles3", false),
    boss("Drox", "AtlasExiles4", false),
];

/// The boss kind of the config's "Is the enemy a Boss?" (`enemyIsBoss`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossKind {
    /// Not a boss (`None`).
    None,
    /// A standard boss (`Boss`).
    Boss,
    /// A Guardian or Pinnacle boss (`Pinnacle`), PoB's default.
    Pinnacle,
    /// An Uber Pinnacle boss (`Uber`).
    Uber,
}

impl BossKind {
    /// The kind of a config value.
    #[must_use]
    pub fn of(value: &str) -> Option<Self> {
        match value {
            "None" => Some(Self::None),
            "Boss" => Some(Self::Boss),
            "Pinnacle" => Some(Self::Pinnacle),
            "Uber" => Some(Self::Uber),
            _ => None,
        }
    }
}

/// PoB's `data.misc.MaxEnemyLevel`.
pub const MAX_ENEMY_LEVEL: i64 = 85;

/// The last level of PoB's monster tables (100 rows).
pub const MAX_TABLE_LEVEL: i64 = 100;

/// The config inputs the enemy model reads (PoB's names), each modelled
/// when its value has the expected type (`enemyIsBoss` a string, the
/// others numbers).
pub const ENEMY_INPUTS: [&str; 8] = [
    "enemyIsBoss",
    "enemyLevel",
    "enemyFireResist",
    "enemyColdResist",
    "enemyLightningResist",
    "enemyChaosResist",
    "enemyArmour",
    "enemyEvasion",
];

/// Why part of the enemy is not known.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnemyProblem {
    /// `enemyIsBoss` has a value PoB's list does not have (PoB then sets
    /// no placeholder, and a build's saved ones would count).
    UnknownBossKind {
        /// The value.
        value: String,
    },
    /// An enemy input has a value of another type.
    InputType {
        /// The input.
        input: &'static str,
    },
    /// A table row the model reads is not in the data.
    MissingData {
        /// The table.
        table: &'static str,
        /// The row.
        key: String,
    },
}

/// The enemy PoB's config describes (Phase 3 plan, step 11): its level,
/// its resistances as configured (before its maximum, penetration, and
/// exposure, which the hits apply), its base armour and evasion.
#[derive(Debug, Clone, PartialEq)]
pub struct Enemy {
    /// The boss kind.
    pub kind: BossKind,
    /// The enemy's level.
    pub level: i64,
    /// Fire, cold, lightning, and chaos resistance (`None`: no config value
    /// and no placeholder, as for a non-boss: PoB adds no modifier).
    pub resistances: [Option<f64>; 4],
    /// Base armour.
    pub armour: Option<f64>,
    /// Base evasion.
    pub evasion: Option<f64>,
    /// What is not known.
    pub problems: Vec<EnemyProblem>,
}

/// The value of config input `name` in the active set, as a number or a
/// string.
enum Value<'a> {
    Number(f64),
    Text(&'a str),
    Other,
}

fn input<'a>(build: &'a Build, name: &str) -> Option<Value<'a>> {
    let config = &build.config;
    let set = config
        .active
        .and_then(|id| config.sets.typed().find(|s| s.id == id))
        .or_else(|| config.sets.typed().next())?;
    let i = set.inputs.typed().find(|i| i.name == name)?;
    Some(match &i.value {
        ConfigValue::Number(n) => Value::Number(n.value().0),
        ConfigValue::String(s) => Value::Text(s.as_str()),
        ConfigValue::Boolean(_) => Value::Other,
    })
}

/// PoB's monster armour table (`src/Export/Scripts/miscdata.lua`: "The
/// table here is wrong so we generate it instead"): `floor((10 + 2 x
/// level) x (1 + MonsterDamageReductionImprovement / 100)^level)`, the
/// constant as value / divisor. `None` without the constant.
#[must_use]
pub fn monster_armour(data: &dyn CalcData, level: i64) -> Option<f64> {
    if !(1..=MAX_TABLE_LEVEL).contains(&level) {
        return None;
    }
    let (value, divisor) = data.game_constant("MonsterDamageReductionImprovement")?;
    let n = u32::try_from(level).ok()?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "game constants and levels are small integers, exact as f64"
    )]
    let (value, divisor, level) = (value as f64, divisor as f64, level as f64);
    // Lua: `Value / Divisor / 100`, left to right.
    let base = 1.0 + value / divisor / 100.0;
    Some(math::floor((10.0 + 2.0 * level) * math::pow_int(base, n)))
}

/// PoB's boss mean (`data.bossStats`): `100 + total / count` of the
/// armour (or evasion) multipliers over [`BOSSES`], or its uber ones.
fn boss_mean(
    data: &dyn CalcData,
    uber: bool,
    armour: bool,
    problems: &mut Vec<EnemyProblem>,
) -> Option<f64> {
    let mut total = 0.0;
    let mut count = 0.0;
    for b in BOSSES.iter().filter(|b| !uber || b.uber) {
        let Some(m) = data.monster_type(b.monster_type) else {
            problems.push(EnemyProblem::MissingData {
                table: "monster_types",
                key: b.monster_type.to_owned(),
            });
            return None;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "multipliers are small integers, exact as f64"
        )]
        let v = if armour { m.armour } else { m.evasion } as f64;
        total += v;
        count += 1.0;
    }
    Some(100.0 + total / count)
}

/// The enemy of `build`'s active config set (module docs; PoB
/// `Classes/ConfigTab.lua` `BuildModList` and `UpdateLevel`,
/// `Modules/ConfigOptions.lua` `enemyIsBoss`, `Modules/CalcSetup.lua`
/// `env.enemyLevel`). Options apply in PoB's list order: the boss kind
/// sets the placeholders of the enemy's level, resistances, armour, and
/// evasion, and an input, when set, wins over its placeholder. A build's
/// saved placeholders are overwritten by the boss kind's.
#[must_use]
pub fn enemy(build: &Build, data: &dyn CalcData) -> Enemy {
    let mut problems = Vec::new();
    let number = |name: &'static str, problems: &mut Vec<EnemyProblem>| match input(build, name) {
        Some(Value::Number(n)) => Some(n),
        Some(_) => {
            problems.push(EnemyProblem::InputType { input: name });
            None
        }
        None => None,
    };
    let kind = match input(build, "enemyIsBoss") {
        None => BossKind::Pinnacle,
        // An unknown kind: PoB sets no placeholder (a build's saved ones
        // would count); the Pinnacle values are only a guess, and the
        // problem says so.
        Some(Value::Text(t)) => BossKind::of(t).unwrap_or_else(|| {
            problems.push(EnemyProblem::UnknownBossKind {
                value: t.to_owned(),
            });
            BossKind::Pinnacle
        }),
        Some(_) => {
            problems.push(EnemyProblem::InputType {
                input: "enemyIsBoss",
            });
            BossKind::Pinnacle
        }
    };
    // `UpdateLevel`: the input when above 0, else the placeholder the boss
    // kind set (84, 85; none for the others), else the character's level,
    // each capped at `MaxEnemyLevel`.
    let character = build
        .character
        .level
        .as_ref()
        .map_or(1, |l| i64::from(*l.value()))
        .clamp(1, 100);
    // A fractional level input is cut to its whole part; PoB keeps it and
    // finds no table row (undefined in PoB, a nil placeholder).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a level input is a small whole number; PoB reads it as one"
    )]
    let input_level = number("enemyLevel", &mut problems)
        .filter(|n| *n > 0.0)
        .map(|n| n as i64);
    let placeholder_level = match kind {
        BossKind::None | BossKind::Boss => None,
        BossKind::Pinnacle => Some(84),
        BossKind::Uber => Some(85),
    };
    let level = input_level
        .or(placeholder_level)
        .unwrap_or(character)
        .min(MAX_ENEMY_LEVEL);
    // The level the armour and evasion placeholders read: the enemy's,
    // at least the boss kind's.
    let table_level = placeholder_level.map_or(level, |p| level.max(p));

    let resist_placeholders: [Option<f64>; 4] = match kind {
        BossKind::None => [None; 4],
        BossKind::Boss => [Some(40.0), Some(40.0), Some(40.0), Some(25.0)],
        BossKind::Pinnacle | BossKind::Uber => [Some(50.0), Some(50.0), Some(50.0), Some(30.0)],
    };
    let names = [
        "enemyFireResist",
        "enemyColdResist",
        "enemyLightningResist",
        "enemyChaosResist",
    ];
    let mut resistances = [None; 4];
    for (i, name) in names.into_iter().enumerate() {
        resistances[i] = number(name, &mut problems).or(resist_placeholders[i]);
    }

    let armour_table = monster_armour(data, table_level);
    if armour_table.is_none() {
        problems.push(EnemyProblem::MissingData {
            table: "game_constants",
            key: "MonsterDamageReductionImprovement".to_owned(),
        });
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "evasion values are far below 2^53"
    )]
    let evasion_table = match data.monster_level(table_level) {
        Some(m) => Some(m.evasion as f64),
        None => {
            problems.push(EnemyProblem::MissingData {
                table: "monster_levels",
                key: table_level.to_string(),
            });
            None
        }
    };
    let scaled =
        |table: Option<f64>, uber: bool, armour: bool, problems: &mut Vec<EnemyProblem>| {
            let mean = boss_mean(data, uber, armour, problems)?;
            Some(math::round_half_up(table? * (mean / 100.0)))
        };
    let (armour_placeholder, evasion_placeholder) = match kind {
        BossKind::None | BossKind::Boss => (armour_table, evasion_table),
        BossKind::Pinnacle => (
            scaled(armour_table, false, true, &mut problems),
            scaled(evasion_table, false, false, &mut problems),
        ),
        BossKind::Uber => (
            scaled(armour_table, true, true, &mut problems),
            scaled(evasion_table, true, false, &mut problems),
        ),
    };
    let armour = number("enemyArmour", &mut problems).or(armour_placeholder);
    let evasion = number("enemyEvasion", &mut problems).or(evasion_placeholder);
    problems.sort();
    problems.dedup();
    Enemy {
        kind,
        level,
        resistances,
        armour,
        evasion,
        problems,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn the_boss_list_should_name_each_boss_and_type_once() {
        let names: BTreeSet<_> = BOSSES.iter().map(|b| b.name).collect();
        let types: BTreeSet<_> = BOSSES.iter().map(|b| b.monster_type).collect();
        assert_eq!(names.len(), BOSSES.len());
        assert_eq!(types.len(), BOSSES.len());
        assert_eq!(BOSSES.iter().filter(|b| b.uber).count(), 7);
    }
}
