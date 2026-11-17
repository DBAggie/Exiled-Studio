//! PoB's hand-written skill data (Phase 6 plan, decision 5): what Path of
//! Building v2.67.2 writes by hand for each skill in `Data/Skills/*.lua`
//! and the game data does not carry: the skill's base flags
//! (`baseFlags`), its parts (`parts`), its base mods (`baseMods`), and the
//! names of its skill functions (`preDamageFunc` and kin, ported by hand
//! per skill). The table itself is generated ([`super::pob_skills`]) by
//! `loom-parity-cli pob-skills` from the PoB data export
//! (`tools/oracle/data/pob_data.lua`, format 10); PoB is MIT licensed.

use super::pob_skills::SKILLS;

/// A plain value of PoB's data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// A boolean.
    Bool(bool),
    /// A number.
    Num(f64),
    /// A string.
    Str(&'static str),
}

impl Value {
    /// The number, a boolean as 1 or 0.
    #[must_use]
    pub const fn num(self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(n),
            Self::Bool(true) => Some(1.0),
            Self::Bool(false) => Some(0.0),
            Self::Str(_) => None,
        }
    }

    /// Whether the value is `true` or a number other than 0.
    #[must_use]
    pub fn truthy(self) -> bool {
        match self {
            Self::Bool(b) => b,
            Self::Num(n) => n != 0.0,
            Self::Str(_) => true,
        }
    }
}

/// One part of a skill (PoB's `parts[i]`): its plain fields (`name`,
/// `area`, `melee`, `projectile`, `stages`, `cast`, `spell`, ...), sorted
/// by key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PobPart {
    /// The fields.
    pub values: &'static [(&'static str, Value)],
}

impl PobPart {
    /// A field's value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<Value> {
        self.values.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
    }

    /// The part's name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self.get("name") {
            Some(Value::Str(s)) => s,
            _ => "",
        }
    }
}

/// One of a skill's base mods (PoB's `baseMods`): a mod's name, type, and
/// plain value; for `SkillData` (`skill("key", value)`), the key and value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PobBaseMod {
    /// The mod's name (`SkillData`, `Damage`, `dotIsBurningGround`, ...).
    pub name: &'static str,
    /// The mod's type (`LIST`, `FLAG`, `BASE`, `MORE`, ...).
    pub kind: &'static str,
    /// For `SkillData`, the key it sets.
    pub key: Option<&'static str>,
    /// The value, when plain.
    pub value: Option<Value>,
    /// PoB's `ModFlag` bits.
    pub flags: u64,
    /// PoB's `KeywordFlag` bits.
    pub keyword_flags: u64,
    /// Whether the mod has tags (conditions): such a mod is read only by
    /// code that models its tags.
    pub tagged: bool,
}

/// A skill's hand-written data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PobSkill {
    /// The granted effect's ID (PoB's skill key, the game's
    /// `GrantedEffects.Id`).
    pub id: &'static str,
    /// The base flags set to `true`, sorted (`area`, `attack`, `spell`, ...).
    pub flags: &'static [&'static str],
    /// The parts, in PoB's order.
    pub parts: &'static [PobPart],
    /// The base mods, in PoB's order.
    pub base_mods: &'static [PobBaseMod],
    /// The names of its skill functions, sorted.
    pub functions: &'static [&'static str],
}

impl PobSkill {
    /// Whether a base flag is set.
    #[must_use]
    pub fn flag(&self, name: &str) -> bool {
        self.flags.binary_search(&name).is_ok()
    }

    /// The value a `SkillData` base mod sets for `key` (`skill("dotIsArea",
    /// true)`).
    #[must_use]
    pub fn skill_data(&self, key: &str) -> Option<Value> {
        self.base_mods
            .iter()
            .find(|m| m.name == "SkillData" && m.key == Some(key) && !m.tagged)
            .and_then(|m| m.value)
    }

    /// Whether an untagged `FLAG` base mod of `name` is set
    /// (`flag("dotIsBurningGround")`).
    #[must_use]
    pub fn base_flag(&self, name: &str) -> bool {
        self.base_mods.iter().any(|m| {
            m.name == name && m.kind == "FLAG" && !m.tagged && m.value.is_some_and(Value::truthy)
        })
    }

    /// Whether the skill has a skill function of `name`.
    #[must_use]
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.binary_search(&name).is_ok()
    }
}

/// The skill's hand-written data, if PoB has any for it.
#[must_use]
pub fn skill(id: &str) -> Option<&'static PobSkill> {
    SKILLS
        .binary_search_by(|s| s.id.cmp(id))
        .ok()
        .and_then(|i| SKILLS.get(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_should_be_sorted_with_sorted_flags() {
        assert!(
            SKILLS
                .windows(2)
                .all(|w| matches!(w, [a, b] if a.id < b.id))
        );
        for s in SKILLS {
            assert!(
                s.flags.windows(2).all(|w| matches!(w, [a, b] if a < b)),
                "{}",
                s.id
            );
            assert!(
                s.functions.windows(2).all(|w| matches!(w, [a, b] if a < b)),
                "{}",
                s.id
            );
            for p in s.parts {
                assert!(
                    p.values
                        .windows(2)
                        .all(|w| matches!(w, [a, b] if a.0 < b.0)),
                    "{}",
                    s.id
                );
            }
        }
    }

    #[test]
    fn smite_should_have_pobs_parts() {
        let smite = skill("Smite").expect("Smite");
        assert!(smite.flag("melee") && smite.flag("area") && smite.flag("attack"));
        let names: Vec<_> = smite.parts.iter().map(PobPart::name).collect();
        assert_eq!(names, ["Melee Hit", "Area Hit"]);
        assert_eq!(
            smite.parts.get(1).and_then(|p| p.get("melee")),
            Some(Value::Bool(false))
        );
        assert!(skill("NoSuchSkill").is_none());
    }

    /// The entries P6-1 reads, pinned against PoB v2.67.2's skill data
    /// (`Data/Skills/act_int.lua`), so a regenerated table that loses them
    /// fails here.
    #[test]
    fn the_dot_skills_should_have_pobs_data() {
        for (id, flags, dot_is_area, function) in [
            (
                "Vortex",
                &["area", "duration", "forceInstant", "spell"][..],
                true,
                true,
            ),
            ("RighteousFire", &["area", "spell"][..], true, true),
            ("VaalRighteousFire", &["area", "spell"][..], true, true),
            (
                "EssenceDrain",
                &["duration", "projectile", "spell"][..],
                false,
                false,
            ),
        ] {
            let s = skill(id).expect(id);
            assert_eq!(s.flags, flags, "{id}");
            assert_eq!(
                s.skill_data("dotIsArea").is_some_and(Value::truthy),
                dot_is_area,
                "{id}"
            );
            assert_eq!(s.has_function("preDamageFunc"), function, "{id}");
        }
        assert!(
            skill("HeraldOfAsh")
                .expect("Herald of Ash")
                .base_flag("dotIsHeraldOfAsh")
        );
        assert!(skill("Swordstorm").is_some_and(|s| {
            s.skill_data("doubleHitsWhenDualWielding")
                .is_some_and(Value::truthy)
        }));
    }
}
