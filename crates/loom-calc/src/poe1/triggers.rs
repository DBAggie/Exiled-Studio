//! Triggers (Phase 6 P6-8a): a triggered skill's rate as PoB v2.67.2
//! computes it (`Modules/CalcTriggers.lua`, `defaultTriggerHandler` 390
//! to 880 and its configs 896 to 1570).
//!
//! **Which trigger** (`calcs.triggers`, 1592 to 1617): PoB looks the config
//! up by the skill's id, its name, then the name of the trigger support
//! that applies to it (`triggeredBy`). The engine ports these configs
//! ([`Config`]):
//! - **Cast On Critical Strike** (1151): the source is the attack in the
//!   same group with the highest rate; the triggered skills are the group's
//!   spells it supports; the trigger chance is the source's hit chance
//!   times its crit chance.
//! - **Cast when Damage Taken** (1175), **Cast when Stunned** (1190), and
//!   **Cast on Ward Break** (1195): global triggers whose source is the
//!   skill itself; the rate is the trigger's cooldown cap.
//! - **Cast on Melee Kill** (1155, P6-8a part 2): as Cast On Critical
//!   Strike from the group's melee attacks, without the crit chance, only
//!   while the character has killed recently (PoB's
//!   `Condition:KilledRecently`: the config's check or its kill count);
//!   else self-cast.
//! - **Cast while Channelling** (1393, `CWCHandler` 219 to 388): the
//!   source is the group's channelling skill the support's first part
//!   supports, its interval the part's `cast_while_channelling_time_ms`
//!   (PoB's `triggerTime`); the rate is the rotation of the group's spells
//!   it triggers at `1 / interval`, without a trigger cooldown or chance.
//!
//! Cast on Death is not ported: PoB's config returns none, but the
//! support's `triggeredOnDeath` changes the skill's own time and damage
//! (`CalcOffence.lua` 2403, 4909), so a skill it supports stays not
//! computed.
//!
//! The Holy Relic's Nova (1167, a minion skill the character's attacks
//! trigger) waits for the skills' level requirements in the bundle: PoB
//! picks a minion skill's level by them (`CalcActiveSkill.lua` 895 to
//! 902), so the minion's skill is not computed yet.
//!
//! A skill with another trigger (a trigger support or an inbuilt trigger
//! not listed) is not computed ([`Kind::Unported`]).
//!
//! **The handler** (390 to 880):
//! 1. **The source:** each candidate is computed as its own skill (PoB's
//!    `calcs.buildActiveSkill`; the engine runs [`crate::calculate`] on a
//!    copy of the build with the candidate as its main skill,
//!    [`super::skill::with_main_skill`]); the highest `HitSpeed` or `Speed`
//!    wins, the first on a tie (`findTriggerSkill`, 74 to 87). A triggered
//!    skill, a disabled one, and the triggered skill itself never are. No
//!    source: PoB reports the skill as self-cast ([`Outcome::SelfCast`]).
//! 2. **The cooldowns:** the trigger's (the trigger support's level
//!    cooldown) and the triggered skill's, each rounded up to a server
//!    tick unless the skill stores more than one use; the cap is `1 /
//!    max(both)` (`TriggerRateCap`, 575 to 585).
//! 3. **The effective source rate:** the source's rate; a global trigger's
//!    is the cap (714 to 719).
//! 4. **The trigger chance** (727 to 790): times the source's hit chance
//!    when it is an attack and not 100, times its crit chance for a trigger
//!    on crit when not 100, times the config's chance when not 100.
//! 5. **The trigger rate** (793 to 860): `min(cap, rate)` for a skill that
//!    stores more than one use and is the only one triggered; the
//!    effective rate for a global trigger; else PoB's simulation of the
//!    triggered skills' rotation ([`rotation_rate`],
//!    `calcMultiSpellRotationImpact`, 87 to 133).
//! 6. **`Speed`** is the trigger rate (`CalcOffence.lua` 2223 to 2227).
//!
//! **Not modelled:** cooldown recovery, added cooldown, and added cast
//! time (each from a stat the mapping does not know, so a reason), dual
//! wielding's halved rate (only for item triggers and item-granted
//! supports, none ported), Unleash, projectiles that hit several times,
//! and trigger-related damage (`TriggeredDamage`: unmapped stats).

use super::Poe1Quantity;
use super::skill::{self, GemEffectRef, MainSkill};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::quantity::{Op, Quantity};
use crate::result::Field;
use crate::source::Source;
use crate::state::{FieldState, Reasons, UnsupportedReason};

/// The state of PoB's `TriggeredDamage` turned into `Damage` (P6-8a part
/// 2: Cast while Channelling's less damage): settled by the spell path
/// from [`Outcome::Rate`]'s flag before the damage
/// (`mapping::STATE_ROWS`).
pub const TRIGGERED_DAMAGE: &str = "TriggeredDamage";

/// PoB's `data.misc.ServerTickTime`.
const SERVER_TICK_TIME: f64 = 0.033;

/// A trigger config the engine ports (module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Config {
    /// Cast On Critical Strike (and its awakened gem).
    CastOnCrit,
    /// Cast when Damage Taken.
    CastWhenDamageTaken,
    /// Cast when Stunned.
    CastWhenStunned,
    /// Cast on Ward Break.
    CastOnWardBreak,
    /// Cast on Melee Kill.
    CastOnMeleeKill,
    /// Cast while Channelling (and its awakened gem).
    CastWhileChannelling,
    /// Spellslinger's support part (open items S3; `CalcTriggers.lua` 1200
    /// to 1212): the source is any wand projectile attack, the trigger on
    /// use (no hit chance), the rotation the main skill alone.
    Spellslinger,
}

/// The trigger supports ported, by the granted effect PoB marks
/// `isTrigger` (`Data/Skills/sup_*.lua`).
const SUPPORTS: &[(&str, Config)] = &[
    ("SupportCastOnCritTriggered", Config::CastOnCrit),
    ("SupportCastOnCritTriggeredPlus", Config::CastOnCrit),
    ("SupportCastWhenDamageTaken", Config::CastWhenDamageTaken),
    ("SupportCastWhenStunned", Config::CastWhenStunned),
    ("SupportCastOnWardBreak", Config::CastOnWardBreak),
    ("SupportCastOnMeleeKillTriggered", Config::CastOnMeleeKill),
    (
        "SupportCastWhileChannellingTriggered",
        Config::CastWhileChannelling,
    ),
    (
        "SupportCastWhileChannellingTriggeredPlus",
        Config::CastWhileChannelling,
    ),
    ("SupportSpellslinger", Config::Spellslinger),
];

/// The first parts of the two-part trigger gems, on the source skill:
/// Cast On Critical Strike's and Cast on Melee Kill's on the attack, Cast
/// while Channelling's on the channelling skill.
const SOURCE_PARTS: &[(&str, Config)] = &[
    ("SupportAwakenedCastOnCriticalStrike", Config::CastOnCrit),
    (
        "SupportAwakenedCastWhileChannelling",
        Config::CastWhileChannelling,
    ),
    ("SupportCastOnCriticalStrike", Config::CastOnCrit),
    ("SupportCastOnMeleeKill", Config::CastOnMeleeKill),
    ("SupportCastWhileChannelling", Config::CastWhileChannelling),
];

/// Cast while Channelling's interval on the channelling skill (PoB's
/// `triggerTime`, `div = 1000`, `Data/Skills/sup_int.lua` 689).
const CWC_TIME_STAT: &str = "cast_while_channelling_time_ms";

/// PoB's trigger supports (`isTrigger = true`, `Data/Skills/sup_*.lua`):
/// a skill one of them supports is triggered.
const TRIGGER_SUPPORTS: &[&str] = &[
    "SupportCastOnCritTriggered",
    "SupportCastOnCritTriggeredPlus",
    "SupportCastOnDeath",
    "SupportCastOnMeleeKillTriggered",
    "SupportCastOnWardBreak",
    "SupportCastWhenDamageTaken",
    "SupportCastWhenStunned",
    "SupportCastWhileChannellingTriggered",
    "SupportCastWhileChannellingTriggeredPlus",
    "SupportCurseOnHitCurse",
    "SupportCurseOnHitCursePlus",
    "SupportManaforgedArrows",
    "SupportMarkOnHit",
    "SupportSpellslinger",
];

/// The stat that makes Cast when Damage Taken's config apply (PoB's
/// `triggeredByDamageTaken`, `SkillStatMap.lua` 258).
const DAMAGE_TAKEN_STAT: &str = "cast_on_damage_taken_threshold";

/// What triggers a skill.
#[derive(Debug, Clone)]
pub enum Kind<'d> {
    /// Nothing: the skill is not triggered.
    NotTriggered,
    /// A ported config, with its trigger support if any.
    Ported(Config, Option<GemEffectRef<'d>>),
    /// A trigger the engine does not port.
    Unported,
}

/// What triggers `skill` (module docs).
#[must_use]
pub fn kind<'d>(skill: &MainSkill<'d>) -> Kind<'d> {
    if let Some(s) = skill
        .supports
        .iter()
        .find(|s| TRIGGER_SUPPORTS.contains(&s.effect.id.as_str()))
    {
        return SUPPORTS
            .iter()
            .find(|(id, _)| *id == s.effect.id)
            .map_or(Kind::Unported, |(_, c)| Kind::Ported(*c, Some(*s)));
    }
    // A skill PoB marks triggered without a ported config. A skill with
    // only `InbuiltTrigger` and no PoB config is self-cast in PoB, as here.
    if skill.types.contains("Triggered") {
        return Kind::Unported;
    }
    Kind::NotTriggered
}

/// A triggered skill's rate, or why there is none.
#[derive(Debug)]
pub enum Outcome {
    /// Not triggered, or no source: PoB reports the skill as self-cast.
    SelfCast,
    /// The trigger rate, per second, with the reasons it carries (the
    /// source's fields'), and whether the trigger's source is the skill
    /// itself (Cast while Channelling, the global triggers): PoB then
    /// turns the skill's `TriggeredDamage` into `Damage`
    /// (`addTriggerIncMoreMods`, 23 to 30, its tag `SkillType.Triggered`
    /// read in the source's config) ([`TRIGGERED_DAMAGE`]).
    Rate(f64, Reasons, bool),
    /// The rate cannot be computed.
    Unsupported(UnsupportedReason),
}

/// One skill of a trigger's rotation (PoB's `packageSkillDataForSimulation`).
struct Rotated {
    /// The skill's own cooldown, seconds.
    cooldown: f64,
    /// Whether it is the main skill.
    main: bool,
    /// Its cooldown recovery factor (PoB's `icdr`).
    icdr: f64,
}

/// The rate of the main skill `skill` when a ported trigger triggers it
/// (module docs).
#[must_use]
pub fn rate(calc: &Calc<'_>, skill: &MainSkill<'_>, source: &Source, flags: Flags) -> Outcome {
    let Kind::Ported(config, support) = kind(skill) else {
        return Outcome::SelfCast;
    };
    let input = *calc.input();
    let levels = super::gem_levels::GemLevels::from_effects(calc.store().effects());
    let unsupported = |what| {
        Outcome::Unsupported(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what,
        })
    };
    let instance_stat = |s: &GemEffectRef<'_>, stat: &str| {
        skill::instance(s.effect, s.level, s.quality, input.data)
            .stats
            .iter()
            .find(|(id, _)| id == stat)
            .map(|(_, v)| *v)
    };
    // Cast on Melee Kill applies only after a kill (its config, 1155).
    if config == Config::CastOnMeleeKill {
        match killed_recently(&input) {
            Some(true) => {}
            Some(false) => return Outcome::SelfCast,
            None => return unsupported("killed-recently"),
        }
    }
    // Cast when Damage Taken applies only with its threshold
    // (`triggeredByDamageTaken`); else PoB reports self-cast.
    if config == Config::CastWhenDamageTaken
        && support
            .as_ref()
            .and_then(|s| instance_stat(s, DAMAGE_TAKEN_STAT))
            .is_none()
    {
        return Outcome::SelfCast;
    }
    let seconds = |ms: i64| {
        #[expect(clippy::cast_precision_loss, reason = "milliseconds, exact as f64")]
        let s = ms as f64 / 1000.0;
        s
    };
    let level_row = skill::effect_level(skill.effect(), skill.active_gem.level);
    let stored_uses = level_row.map_or(0, |l| l.stored_uses);
    let triggered_cd = level_row.map_or(0.0, |l| seconds(l.cooldown));
    let trigger_cd = support
        .as_ref()
        .and_then(|s| skill::effect_level(s.effect, s.level))
        .map_or(0.0, |l| seconds(l.cooldown));
    let mut reasons = Reasons::new();
    // The triggered skill's cooldown recovery (open items S3; PoB's
    // `icdr = calcLib.mod(skillModList, skillCfg, "CooldownRecovery")`,
    // 558): both cooldowns are divided by it (575 to 576), in the cap and
    // in the rotation (`packageSkillDataForSimulation`, 64).
    let Some(icdr) = cooldown_recovery(calc, flags, &mut reasons) else {
        return unsupported("cooldown-recovery");
    };
    // PoB's `ignoresTickRate`: more than one stored use.
    let ignores_tick = stored_uses > 1;
    let tick_rate = 1.0 / SERVER_TICK_TIME;
    let rounded = |cd: f64, ignore: bool| {
        if ignore {
            cd
        } else {
            crate::math::ceil(cd * tick_rate) / tick_rate
        }
    };
    let action = crate::math::max(
        rounded(trigger_cd / icdr, false),
        rounded(triggered_cd / icdr, ignores_tick),
    );
    let cap = if action == 0.0 {
        f64::INFINITY
    } else {
        1.0 / action
    };
    let global = matches!(
        config,
        Config::CastWhenDamageTaken | Config::CastWhenStunned | Config::CastOnWardBreak
    );
    if global {
        // `skillFlags.globalTrigger` without `triggeredSkillCond`: the
        // effective rate is the cap, and the trigger rate the effective
        // rate (or `min(cap, rate)` when it ignores the tick rate).
        if !cap.is_finite() {
            return unsupported("trigger-no-cooldown");
        }
        return Outcome::Rate(cap, reasons, true);
    }
    let skills = skill::active_skills(input.build, input.data, &levels);
    let main_group = skill.group;
    let is_main = |s: &MainSkill<'_>| {
        s.active_gem.gem == skill.active_gem.gem && s.effect().id == skill.effect().id
    };
    let rotation_of = |config: Config| -> Vec<Rotated> {
        skills
            .iter()
            .filter(|(g, _, s)| {
                *g == main_group
                    && s.types.contains("Spell")
                    && s.supports.iter().any(|x| {
                        SUPPORTS
                            .iter()
                            .any(|(id, c)| *c == config && *id == x.effect.id)
                    })
            })
            .map(|(_, _, s)| Rotated {
                cooldown: skill::effect_level(s.effect(), s.active_gem.level)
                    .map_or(0.0, |l| seconds(l.cooldown)),
                main: is_main(s),
                // The main skill's factor for each: the engine computes
                // the main skill's modifiers only. The rotations ported
                // share one trigger support, whose recovery is the same
                // on each.
                icdr,
            })
            .collect()
    };
    if config == Config::CastWhileChannelling {
        // `CWCHandler`: the first channelling skill of the group the
        // support's first part supports, not triggered (219 to 236).
        let interval = skills.iter().find_map(|(g, _, s)| {
            if *g != main_group || is_main(s) || s.types.contains("Triggered") {
                return None;
            }
            s.supports
                .iter()
                .find(|x| {
                    SOURCE_PARTS
                        .iter()
                        .any(|(id, c)| *c == Config::CastWhileChannelling && *id == x.effect.id)
                })
                .and_then(|x| instance_stat(x, CWC_TIME_STAT))
        });
        let rotation = rotation_of(Config::CastWhileChannelling);
        let Some(ms) = interval.filter(|ms| *ms > 0) else {
            return Outcome::SelfCast;
        };
        if !rotation.iter().any(|r| r.main) {
            return Outcome::SelfCast;
        }
        return Outcome::Rate(
            rotation_rate(&rotation, 1.0 / seconds(ms), 0.0, 100.0),
            reasons,
            true,
        );
    }
    // The source (`findTriggerSkill`).
    let player_main = skill::main_skill(input.build, input.data, &levels).ok();
    let is_player_main = |g: usize, s: &MainSkill<'_>| {
        player_main.as_ref().is_some_and(|m| {
            m.group == g && m.active_gem.gem == s.active_gem.gem && m.effect().id == s.effect().id
        })
    };
    let hands = super::collect::hands(&input);
    // Spellslinger's source: a projectile attack with the wand flag (PoB's
    // `ModFlag.Wand` in the attack's `skillCfg`: the main hand holds a
    // wand), in any group (1209 to 1210).
    let wand = matches!(&hands.main_hand, super::collect::Held::Weapon(c) if c == "Wand");
    let mut best: Option<Source3> = None;
    for (g, a, s) in &skills {
        let candidate = s.types.contains("Attack")
            && !s.types.contains("Triggered")
            && !s.types.contains("InbuiltTrigger")
            && !s.types.contains("OtherThingUsesSkill")
            && !is_player_main(*g, s)
            && match config {
                Config::CastOnCrit => *g == main_group,
                Config::CastOnMeleeKill => *g == main_group && s.types.contains("Melee"),
                Config::Spellslinger => wand && s.types.contains("Projectile"),
                Config::CastWhenDamageTaken
                | Config::CastWhenStunned
                | Config::CastOnWardBreak
                | Config::CastWhileChannelling => false,
            };
        if !candidate || s.disabled_by_hands(&hands) != Ok(false) {
            continue;
        }
        let b = skill::with_main_skill(input.build, *g, *a);
        let mut options = input.options.clone();
        options.nested = true;
        let sub = crate::input::CalcInput {
            build: &b,
            data: input.data,
            options: &options,
        };
        let Ok(result) = crate::calculate(&sub) else {
            return unsupported("trigger-source");
        };
        let speed = result
            .fields
            .get(&Field::HitSpeed)
            .filter(|f| f.value.is_some())
            .or_else(|| result.fields.get(&Field::Speed));
        let Some(rate) = speed.and_then(|f| f.value) else {
            return unsupported("trigger-source");
        };
        if best.as_ref().is_none_or(|b| rate > b.rate) {
            best = Some(Source3 {
                rate,
                speed: speed.map(|f| f.state.clone()),
                hit: result
                    .fields
                    .get(&Field::HitChance)
                    .map(|f| (f.value, f.state.clone())),
                crit: result
                    .fields
                    .get(&Field::CritChance)
                    .map(|f| (f.value, f.state.clone())),
            });
        }
    }
    let Some(src) = best else {
        return Outcome::SelfCast;
    };
    let mut carry = |state: Option<&FieldState>| state.is_none_or(|s| carry(s, &mut reasons));
    if !carry(src.speed.as_ref()) {
        return unsupported("trigger-source");
    }
    // The trigger chance (727 to 790): the source is an attack; a trigger
    // on use (Spellslinger's `triggerOnUse`, 732) skips its hit chance.
    let on_use = config == Config::Spellslinger;
    let mut chance = 100.0;
    let (hit, hit_state) = src
        .hit
        .clone()
        .unwrap_or((Some(100.0), FieldState::Complete));
    let Some(hit) = hit else {
        return unsupported("trigger-source");
    };
    if hit != 100.0 && !on_use {
        if !carry(Some(&hit_state)) {
            return unsupported("trigger-source");
        }
        chance = chance * hit / 100.0;
    }
    if config == Config::CastOnCrit {
        let Some((Some(crit), crit_state)) = src.crit.clone() else {
            return unsupported("trigger-source");
        };
        if crit != 100.0 {
            if !carry(Some(&crit_state)) {
                return unsupported("trigger-source");
            }
            chance = chance * crit / 100.0;
        }
    }
    // The rotation: the group's spells the trigger supports
    // (`triggeredByCoc`, `triggeredByMeleeKill`, and `slotMatch`); a
    // config without `triggeredSkillCond` (Spellslinger) simulates the main
    // skill alone (809).
    let rotation = if config == Config::Spellslinger {
        vec![Rotated {
            cooldown: triggered_cd,
            main: true,
            icdr,
        }]
    } else {
        rotation_of(config)
    };
    if !rotation.iter().any(|r| r.main) {
        return unsupported("trigger-rotation");
    }
    let effective = src.rate;
    let value = if ignores_tick && rotation.len() == 1 {
        crate::math::min(cap, effective)
    } else {
        let trigger = if trigger_cd != 0.0 {
            trigger_cd
        } else {
            triggered_cd
        };
        rotation_rate(&rotation, effective, trigger, chance)
    };
    Outcome::Rate(value, reasons, false)
}

/// The main skill's cooldown recovery factor in a context of `flags`: PoB's
/// `calcLib.mod(skillModList, skillCfg, "CooldownRecovery")`, `(1 + inc /
/// 100) x more`, over the character's untagged and typed terms and the
/// skill's own (the quantities a self-cast spell's cooldown reads,
/// `hit::cooldown_limit`); `None` when it is not above 0 (PoB would divide
/// by it).
fn cooldown_recovery(calc: &Calc<'_>, flags: Flags, reasons: &mut Reasons) -> Option<f64> {
    let cx = calc.context(flags);
    let quantities = [
        Quantity::Poe1(Poe1Quantity::SkillTiming(
            super::skill_modifiers::SkillTiming::CooldownRecovery,
        )),
        Quantity::Poe1(Poe1Quantity::TypedCooldownRecovery),
    ];
    let mut inc = 0.0;
    let mut more = 1.0;
    for q in quantities {
        for e in super::repeats::applying(calc, q, Op::Increased, &cx, reasons) {
            inc += e.value;
        }
        for e in super::repeats::applying(calc, q, Op::More, &cx, reasons) {
            more *= 1.0 + e.value / 100.0;
        }
    }
    let icdr = (1.0 + inc / 100.0) * more;
    (icdr > 0.0).then_some(icdr)
}

/// PoB's `Condition:KilledRecently` from the config (`ConfigOptions.lua`:
/// the check, or a kill count above 0 through `multiplierKilledRecently`);
/// `None` for a value of another type.
fn killed_recently(input: &crate::input::CalcInput<'_>) -> Option<bool> {
    let set = super::collect::active_config(input.build);
    let check = super::conditions::check(set, "conditionKilledRecently")?;
    let count = match set.and_then(|s| {
        super::config::last_input(
            s,
            "multiplierKilledRecently",
            loom_domain::ConfigInputKind::Input,
        )
    }) {
        None => false,
        Some(loom_domain::ConfigValue::Number(n)) => n.value().0 > 0.0,
        Some(_) => return None,
    };
    Some(check || count)
}

/// Adds a field's reasons to `reasons` (its partial and indeterminate
/// ones); `false` when the field is unsupported (P6-8a: a trigger's
/// source; P6-8b: a mirage's copy).
pub(super) fn carry(state: &FieldState, reasons: &mut Reasons) -> bool {
    match state {
        FieldState::Complete => true,
        FieldState::Partial(p) => {
            reasons.partial.extend(p.iter().cloned());
            true
        }
        FieldState::Indeterminate {
            reasons: r,
            partial,
        } => {
            reasons.indeterminate.extend(r.iter().cloned());
            reasons.partial.extend(partial.iter().cloned());
            true
        }
        FieldState::Unsupported(_) => false,
    }
}

/// The source skill's fields a trigger reads.
struct Source3 {
    rate: f64,
    speed: Option<FieldState>,
    hit: Option<(Option<f64>, FieldState)>,
    crit: Option<(Option<f64>, FieldState)>,
}

/// PoB's `calcMultiSpellRotationImpact` (`CalcTriggers.lua` 87 to 133):
/// 1,000 source uses at `source_rate`, each triggering the next skill of
/// the rotation off cooldown (its cooldown the larger of its own and the
/// trigger's, from the start of the tick to the next tick after it ends);
/// the main skill's rate is `1 / (time / count + (1 / rate) / chance x 100
/// - 1 / rate)`.
fn rotation_rate(rotation: &[Rotated], source_rate: f64, trigger_cd: f64, chance: f64) -> f64 {
    let increment = 1.0 / source_rate;
    let sim_time = increment * 1000.0;
    let ceil_b = |x: f64| SERVER_TICK_TIME * crate::math::ceil(x / SERVER_TICK_TIME);
    let floor_b = |x: f64| SERVER_TICK_TIME * crate::math::floor(x / SERVER_TICK_TIME);
    // Each skill's cooldown and the trigger's over its recovery (69).
    let cds: Vec<f64> = rotation
        .iter()
        .map(|r| crate::math::max(r.cooldown / r.icdr, trigger_cd / r.icdr))
        .collect();
    let n = rotation.len();
    let mut next_trig = vec![0.0_f64; n];
    let mut count = vec![0_u32; n];
    let mut index = 0;
    let mut next = 0.0_f64;
    while next < sim_time {
        let mut current = index;
        loop {
            if next_trig[current] <= next {
                count[current] += 1;
                next_trig[current] = ceil_b(floor_b(next) + cds[current]);
                break;
            }
            current = (current + 1) % n;
            if current == index {
                break;
            }
        }
        index = (index + 1) % n;
        next += increment;
    }
    rotation
        .iter()
        .zip(&count)
        .find(|(r, _)| r.main)
        .map_or(0.0, |(_, c)| {
            1.0 / (sim_time / f64::from(*c) + (increment / chance * 100.0) - increment)
        })
}

/// Whether the actor's main skill is computed through a trigger the
/// engine ports, or not triggered at all.
#[must_use]
pub fn computable(skill: &MainSkill<'_>) -> bool {
    !matches!(kind(skill), Kind::Unported)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One skill, no trigger cooldown, every source use triggers it: the
    /// source's rate.
    #[test]
    fn a_lone_skill_without_cooldown_should_follow_the_source() {
        let r = rotation_rate(
            &[Rotated {
                cooldown: 0.0,
                main: true,
                icdr: 1.0,
            }],
            2.0,
            0.0,
            100.0,
        );
        assert!((r - 2.0).abs() < 1e-9, "{r}");
    }

    /// Open items S3: Spellslinger's 0.6 s cooldown over its 38% cooldown
    /// recovery and a 2.25 source rate give PoB's 1.197 (PoB v2.67.2's
    /// `SkillTriggerRate` on the Spellslinger fixture with Power Siphon as
    /// the source, `docs/phase-6/triggers.md`, "Part 3").
    #[test]
    fn the_cooldown_recovery_should_divide_the_cooldowns_in_the_rotation() {
        let r = rotation_rate(
            &[Rotated {
                cooldown: 0.0,
                main: true,
                icdr: 1.38,
            }],
            2.25,
            0.6,
            100.0,
        );
        assert!((r - 1.197).abs() < 1e-9, "{r}");
    }

    /// A 0.15 s trigger cooldown caps a fast source near PoB's tick-rounded
    /// cap; two skills share the triggers.
    #[test]
    fn the_rotation_should_share_and_cap_the_triggers() {
        let one = rotation_rate(
            &[Rotated {
                cooldown: 0.0,
                main: true,
                icdr: 1.0,
            }],
            20.0,
            0.15,
            100.0,
        );
        assert!(one > 5.0 && one <= 1.0 / 0.165 + 1e-9, "{one}");
        let two = rotation_rate(
            &[
                Rotated {
                    cooldown: 0.0,
                    main: true,
                    icdr: 1.0,
                },
                Rotated {
                    cooldown: 0.0,
                    main: false,
                    icdr: 1.0,
                },
            ],
            1.0,
            0.15,
            100.0,
        );
        assert!((two - 0.5).abs() < 0.01, "{two}");
    }
}
