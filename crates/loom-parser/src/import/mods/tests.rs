//! The filter over made-up rules: domains, generation types, flags, and
//! tags are synthetic, as for any game.

use loom_data::stat_desc::{DescRef, Hand, StatValue};
use loom_domain::{InfluenceId, ModSection};

use super::*;
use crate::resolve::Candidate;

const OWN: i64 = 1;
const SHARED: i64 = 2;
const AFFIX: i64 = 10;
const SPECIAL: i64 = 20;
const MARKED: i64 = 30;

/// Explicit lines: affixes in the base's domain, or `SHARED` rows on
/// `OWN` bases; `{marked}` implicits: `MARKED` rows; other implicits:
/// `SPECIAL` or `MARKED` rows in the base's domain.
fn rules() -> ModRules {
    ModRules {
        fits: vec![
            FitRule {
                section: ModSection::Explicit,
                flag: None,
                unique: None,
                clauses: vec![FitClause {
                    generations: vec![AFFIX],
                    domains: DomainMatch {
                        own: true,
                        fixed: Vec::new(),
                        related: vec![(SHARED, vec![OWN])],
                    },
                    craft_class: false,
                    essence_class: true,
                }],
            },
            FitRule {
                section: ModSection::Implicit,
                flag: Some("marked".to_owned()),
                unique: None,
                clauses: vec![FitClause {
                    generations: vec![MARKED],
                    domains: DomainMatch {
                        own: true,
                        ..DomainMatch::default()
                    },
                    ..FitClause::default()
                }],
            },
            FitRule {
                section: ModSection::Implicit,
                flag: None,
                unique: None,
                clauses: vec![FitClause {
                    generations: vec![SPECIAL, MARKED],
                    domains: DomainMatch {
                        own: true,
                        ..DomainMatch::default()
                    },
                    ..FitClause::default()
                }],
            },
        ],
        spawn_exclusion: Unflagged {
            section: ModSection::Implicit,
            flags: vec!["marked".to_owned()],
        },
        spawn_excluded_generations: vec![MARKED],
        base_implicit: Unflagged {
            section: ModSection::Implicit,
            flags: vec!["marked".to_owned()],
        },
        no_spawn_flags: vec!["bench".to_owned()],
        sections: Vec::new(),
        local_carriers: LocalCarriers {
            weapon: "arm".to_owned(),
            item: "shell".to_owned(),
        },
        influences: vec![("Sky".to_owned(), 4)],
    }
}

fn row(id: &str, domain: i64, generation: i64, stat: &str, spawn: &[(&str, i64)]) -> ModRow {
    ModRow {
        id: id.to_owned(),
        domain,
        generation,
        stats: vec![(stat.to_owned(), 1, 10)],
        spawn: spawn.iter().map(|(t, w)| ((*t).to_owned(), *w)).collect(),
        ..ModRow::default()
    }
}

fn base(domain: i64, tags: &[&str]) -> BaseRow {
    BaseRow {
        domain,
        tags: Some(tags.iter().map(|t| (*t).to_owned()).collect()),
        ..BaseRow::default()
    }
}

fn line(section: ModSection, flags: &[&str]) -> LineContext {
    LineContext {
        section,
        flags: flags.iter().map(|f| (*f).to_owned()).collect(),
    }
}

fn cand(stat: &str, v: i64) -> Candidate {
    Candidate {
        at: DescRef {
            depth: 0,
            index: 0,
            hand: Hand::Main,
        },
        variant: 0,
        stats: vec![ResolvedStat {
            stat: stat.to_owned(),
            value: RawValue::Exact(StatValue::exact(v)),
        }],
        alternatives: Vec::new(),
    }
}

#[test]
fn fits_should_admit_a_related_domain_on_its_base_domains_only() {
    let r = rules();
    let shared = row("s", SHARED, AFFIX, "a", &[]);
    let explicit = line(ModSection::Explicit, &[]);
    assert_eq!(
        [
            fits(&r, &shared, &base(OWN, &[]), false, &explicit),
            fits(&r, &shared, &base(3, &[]), false, &explicit),
        ],
        [true, false]
    );
}

#[test]
fn fits_should_take_the_first_rule_whose_flag_the_line_carries() {
    let r = rules();
    let special = row("s", OWN, SPECIAL, "a", &[]);
    let b = base(OWN, &[]);
    assert_eq!(
        [
            fits(&r, &special, &b, false, &line(ModSection::Implicit, &[])),
            fits(
                &r,
                &special,
                &b,
                false,
                &line(ModSection::Implicit, &["marked"])
            ),
        ],
        [true, false]
    );
}

#[test]
fn fits_should_admit_no_row_on_a_section_without_a_rule() {
    let r = rules();
    let any = row("s", OWN, AFFIX, "a", &[]);
    let other = line(ModSection::Other("elsewhere".to_owned()), &[]);
    assert!(!fits(&r, &any, &base(OWN, &[]), false, &other));
}

#[test]
fn fits_should_need_the_base_class_for_an_essence_row_with_known_classes() {
    let r = rules();
    let mut essence = row("e", OWN, AFFIX, "a", &[]);
    essence.essence_only = true;
    essence.essence_classes = Some(vec![5]);
    let explicit = line(ModSection::Explicit, &[]);
    let mut on = base(OWN, &[]);
    on.class = Some(5);
    let mut off = base(OWN, &[]);
    off.class = Some(6);
    assert_eq!(
        [
            fits(&r, &essence, &on, false, &explicit),
            fits(&r, &essence, &off, false, &explicit)
        ],
        [true, false]
    );
}

fn ctx<'a>(rules: &'a ModRules, mods: &'a Mods, bases: &'a [BaseRow]) -> Context<'a> {
    Context {
        rules,
        mods,
        bases,
        unique: false,
        influences: Vec::new(),
    }
}

#[test]
fn filter_should_keep_the_candidate_a_fitting_row_backs() {
    let r = rules();
    let mods = Mods::from_rows(vec![
        row("fits", OWN, AFFIX, "a", &[]),
        row("other", 9, AFFIX, "b", &[]),
    ]);
    let bases = [base(OWN, &[])];
    let d = filter(
        &ctx(&r, &mods, &bases),
        &line(ModSection::Explicit, &[]),
        &[cand("a", 5), cand("b", 5)],
    );
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![0], "resolved/mods".to_owned())
    );
}

#[test]
fn filter_should_break_a_tie_by_spawn_tags() {
    let r = rules();
    let mods = Mods::from_rows(vec![
        row("x", OWN, AFFIX, "a", &[("ring", 100)]),
        row("y", OWN, AFFIX, "b", &[("ring", 0)]),
    ]);
    let bases = [base(OWN, &["ring"])];
    let d = filter(
        &ctx(&r, &mods, &bases),
        &line(ModSection::Explicit, &[]),
        &[cand("a", 5), cand("b", 5)],
    );
    assert_eq!(d.name(), "resolved/spawn-tags");
}

#[test]
fn filter_should_skip_spawn_tags_for_a_no_spawn_flag() {
    let r = rules();
    let mods = Mods::from_rows(vec![
        row("x", OWN, AFFIX, "a", &[("ring", 100)]),
        row("y", OWN, AFFIX, "b", &[("ring", 0)]),
    ]);
    let bases = [base(OWN, &["ring"])];
    let d = filter(
        &ctx(&r, &mods, &bases),
        &line(ModSection::Explicit, &["bench"]),
        &[cand("a", 5), cand("b", 5)],
    );
    assert!(d.notes.contains("crafted-no-spawn-tags"));
}

#[test]
fn filter_should_not_count_excluded_generations_as_spawning_on_unflagged_lines() {
    let r = rules();
    let mods = Mods::from_rows(vec![
        row("marked", OWN, MARKED, "a", &[("ring", 100)]),
        row("special", OWN, SPECIAL, "b", &[]),
    ]);
    let bases = [base(OWN, &["ring"])];
    let d = filter(
        &ctx(&r, &mods, &bases),
        &line(ModSection::Implicit, &[]),
        &[cand("a", 5), cand("b", 5)],
    );
    assert!(!d.rules.contains(&"spawn-tags"), "{d:?}");
}

#[test]
fn filter_should_prefer_the_base_implicit_on_unflagged_implicit_lines() {
    let r = rules();
    let mods = Mods::from_rows(vec![
        row("x", OWN, SPECIAL, "a", &[]),
        row("y", OWN, SPECIAL, "b", &[]),
    ]);
    let mut b = base(OWN, &[]);
    b.implicits = vec![1];
    let bases = [b];
    let d = filter(
        &ctx(&r, &mods, &bases),
        &line(ModSection::Implicit, &[]),
        &[cand("a", 5), cand("b", 5)],
    );
    assert_eq!(d.name(), "resolved/base-implicit");
}

#[test]
fn local_stat_should_keep_the_local_reading_on_a_carrier_base() {
    let r = rules();
    let mut mods = Mods::from_rows(vec![
        row("local", OWN, AFFIX, "local_a", &[]),
        row("global", OWN, AFFIX, "global_a", &[]),
    ]);
    mods.locality.insert("local_a".to_owned(), Locality::Item);
    let bases = [base(OWN, &["shell"])];
    let d = filter(
        &ctx(&r, &mods, &bases),
        &line(ModSection::Explicit, &[]),
        &[cand("local_a", 5), cand("global_a", 5)],
    );
    assert_eq!(
        (d.kept.clone(), d.name()),
        (vec![0], "resolved/local-stat".to_owned())
    );
}

#[test]
fn influence_value_should_follow_the_rules() {
    let r = rules();
    assert_eq!(
        [
            r.influence_value(&InfluenceId::new("Sky")),
            r.influence_value(&InfluenceId::new("Sea"))
        ],
        [Some(4), None]
    );
}

#[test]
fn backing_should_name_a_row_of_another_domain_as_not_fitting() {
    let r = rules();
    let mods = Mods::from_rows(vec![row("x", 9, AFFIX, "a", &[])]);
    let bases = [base(OWN, &[])];
    let c = cand("a", 5);
    assert_eq!(
        backing(
            &ctx(&r, &mods, &bases),
            &line(ModSection::Explicit, &[]),
            &c.stats
        ),
        Err("no-fitting-row")
    );
}

#[test]
fn unnumbered_should_strip_a_single_digit_suffix_from_two_to_nine() {
    assert_eq!(
        [unnumbered("x_3"), unnumbered("x_10"), unnumbered("x")],
        [("x", 3), ("x_10", 1), ("x", 1)]
    );
}

#[test]
fn validate_should_accept_the_test_rules() {
    assert_eq!(rules().validate(), Ok(()));
}

#[test]
fn validate_should_refuse_rules_without_fit_rules() {
    let mut r = rules();
    r.fits.clear();
    assert_eq!(
        r.validate().map_err(|e| e.code()),
        Err("IMPORT_MOD_RULES_EMPTY")
    );
}

#[test]
fn validate_should_refuse_a_rule_an_earlier_unflagged_rule_shadows() {
    let mut r = rules();
    let unflagged = r.fits[2].clone();
    r.fits.insert(1, unflagged);
    assert_eq!(r.validate(), Err(ModRulesError::Shadowed(2)));
}

#[test]
fn validate_should_refuse_a_section_name_the_rules_do_not_list() {
    let mut r = rules();
    r.fits[0].section = ModSection::Other("implcit".to_owned());
    assert_eq!(
        r.validate(),
        Err(ModRulesError::UnknownSection("implcit".to_owned()))
    );
}
