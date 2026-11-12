//! The enemy (Phase 3 plan, step 11). P3-1d adds the bosses PoB v2.67.2
//! averages for its boss enemies' armour and evasion; the enemy model
//! itself is P3-11's.
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
