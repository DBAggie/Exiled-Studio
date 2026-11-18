//! PoB's minion templates (Phase 6 P6-7b1): what Path of Building
//! v2.67.2's `Export/Minions/Minions.txt` and `Spectres.txt` add to each
//! game monster variety when its exporter writes `Data/Minions.lua` and
//! `Data/Spectres.lua` (`src/Export/Scripts/minions.lua` 97 to 249): the
//! name PoB's skills use (`minionList`), the variety it maps to, the
//! limit (`#limit`), whether it is hostile (`#hostile`), extra skills, and
//! hand-written mods (`#mod`). The table itself is generated
//! ([`super::pob_minions`]) by `loom-parity-cli pob-minions` from the PoB
//! data export (`tools/oracle/data/pob_data.lua`, format 13); PoB is MIT
//! licensed.
//!
//! The `#mod` extras are kept as PoB wrote them, each cited by its line:
//! they are PoB's, not the game's, so a step that applies one types it with
//! evidence (P6-7b3a for base values).

use super::pob_minions::MINIONS;

/// The template file a minion comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    /// `Export/Minions/Minions.txt`: the minions of skills, by short name.
    Minions,
    /// `Export/Minions/Spectres.txt`: spectres, by variety ID.
    Spectres,
}

/// One `#mod` directive, as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PobMinionMod {
    /// Its line in the template file.
    pub line: u32,
    /// The mod (`mod("Life", "MORE", 698), -- MonsterUnique2`).
    pub text: &'static str,
}

/// One minion PoB's templates emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PobMinion {
    /// PoB's name (the key of `data.minions`): a short name
    /// (`RaisedZombie`) or, for spectres, the variety's ID.
    pub name: &'static str,
    /// The template file.
    pub template: Template,
    /// The `#monster` or `#spectre` line.
    pub line: u32,
    /// The game's monster variety (`MonsterVarieties.Id`).
    pub variety: &'static str,
    /// `#limit`: the stat-derived limit's name (`ActiveZombieLimit`).
    pub limit: Option<&'static str>,
    /// `#hostile`, as written.
    pub hostile: Option<&'static str>,
    /// Skills added after the variety's own, in order.
    pub skills: &'static [&'static str],
    /// The `#mod` extras, in order.
    pub mods: &'static [PobMinionMod],
}

/// The minion PoB's templates emit under `name`, if any.
#[must_use]
pub fn minion(name: &str) -> Option<&'static PobMinion> {
    MINIONS
        .binary_search_by(|m| m.name.cmp(name))
        .ok()
        .and_then(|i| MINIONS.get(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_should_be_sorted_by_name() {
        assert!(
            MINIONS
                .windows(2)
                .all(|w| matches!(w, [a, b] if a.name < b.name))
        );
    }

    /// PoB's Raised Zombie (`Minions.txt`): its variety, limit, and no
    /// extras; Animated Weapon's unique variant has PoB's hand-written
    /// `Life MORE 698`.
    #[test]
    fn known_minions_should_have_pobs_directives() {
        let zombie = minion("RaisedZombie").expect("RaisedZombie");
        assert_eq!(zombie.template, Template::Minions);
        assert_eq!(
            zombie.variety,
            "Metadata/Monsters/RaisedZombies/RaisedZombieStandard"
        );
        assert_eq!(zombie.limit, Some("ActiveZombieLimit"));
        assert!(zombie.mods.is_empty());
        let unique = minion("UniqueAnimatedWeapon").expect("UniqueAnimatedWeapon");
        assert!(
            unique
                .mods
                .iter()
                .any(|m| m.text.starts_with("mod(\"Life\", \"MORE\", 698)"))
        );
        assert!(minion("NoSuchMinion").is_none());
    }

    /// Every name a skill's minion list gives from `Minions.txt` is in the
    /// table (spectres are picked from the build, P6-7c), except the
    /// combined Guardian relic, which PoB writes by hand, not by a
    /// directive (`Export/Minions/Minions.txt` 183).
    #[test]
    fn skills_minion_names_should_be_in_the_table() {
        for skill in super::super::pob_skills::SKILLS {
            for name in skill.minions.iter().chain(skill.add_minions) {
                assert!(
                    minion(name).is_some()
                        || name.starts_with("Metadata/")
                        || *name == "GuardianRelicAll",
                    "{}: {name}",
                    skill.id
                );
            }
        }
    }
}
