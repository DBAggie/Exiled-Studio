//! Unit tests per construct and property tests, over synthetic descriptions
//! (no extract: core crates do no filesystem access, ADR-016).

#![expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]

use std::collections::BTreeMap;

use loom_data::stat_desc::{
    Condition, Description, Hand, Handler, Lookup, LookupHandler, NoLookup, StatDescFile,
    StatDescriptions, StatValue, Template, Variant, VariantFilter, render_description,
};

use super::*;

const ANY: Condition = Condition::Range {
    min: None,
    max: None,
};
const POSITIVE: Condition = Condition::Range {
    min: Some(1),
    max: None,
};
const NEGATIVE: Condition = Condition::Range {
    min: None,
    max: Some(-1),
};
const ZERO: Condition = Condition::Range {
    min: Some(0),
    max: Some(0),
};

fn variant(conditions: &[Condition], text: &str, handlers: &[(&str, usize)]) -> Variant {
    let stats = conditions.len();
    Variant {
        conditions: conditions.to_vec(),
        gem_quality: false,
        table_only: false,
        unknown_flags: Vec::new(),
        handlers: handlers
            .iter()
            .map(|(name, stat)| Handler::classify(name, Some(stat + 1), None, stats).unwrap())
            .collect(),
        line: 1,
        text: text.to_owned(),
        template: Template::parse(text, stats),
    }
}

fn desc(stats: &[&str], variants: Vec<Variant>) -> Description {
    Description {
        name: None,
        line: 1,
        stats: stats.iter().map(|s| (*s).to_owned()).collect(),
        off_hand_stats: None,
        variants,
    }
}

/// One stat, one variant without conditions.
fn simple(stat: &str, text: &str) -> Description {
    desc(&[stat], vec![variant(&[ANY], text, &[])])
}

/// `{0}% increased X` for positive values, `{0}% reduced X` (negated) for
/// negative ones.
fn increased_reduced(stat: &str) -> Description {
    desc(
        &[stat],
        vec![
            variant(&[POSITIVE], "{0}% increased X", &[]),
            variant(&[NEGATIVE], "{0}% reduced X", &[("negate", 0)]),
        ],
    )
}

fn file(source: &str, includes: &[&str], descriptions: Vec<Description>) -> StatDescFile {
    StatDescFile {
        source: source.to_owned(),
        includes: includes.iter().map(|s| (*s).to_owned()).collect(),
        descriptions,
        no_description: Vec::new(),
    }
}

fn set_of(files: Vec<StatDescFile>) -> StatDescriptions {
    let mut set = StatDescriptions::new();
    for f in files {
        set.insert(f);
    }
    set
}

fn one_file(descriptions: Vec<Description>) -> StatDescriptions {
    set_of(vec![file("a.txt", &[], descriptions)])
}

/// Resolves `line` in `a.txt` of `set`.
fn resolve(set: &StatDescriptions, line: &str) -> Resolution {
    let scope = set.scope("a.txt").unwrap();
    Index::build(&scope).resolve(line)
}

fn resolved(r: &Resolution) -> &Candidate {
    match &r.outcome {
        Outcome::Resolved(c) => c,
        other => panic!("not resolved: {other:?}"),
    }
}

fn values(r: &Resolution) -> Vec<(String, RawValue)> {
    resolved(r)
        .stats
        .iter()
        .map(|s| (s.stat.clone(), s.value.clone()))
        .collect()
}

fn exact(v: i64) -> RawValue {
    RawValue::Exact(StatValue::exact(v))
}

fn ranged(min: i64, max: i64) -> RawValue {
    RawValue::Ranged(StatValue { min, max })
}

fn rejections(r: &Resolution) -> Vec<&'static str> {
    match &r.outcome {
        Outcome::Unresolved(Unresolved::NoVariantFits(all)) => {
            all.iter().map(|x| x.reason.name()).collect()
        }
        other => panic!("expected rejections: {other:?}"),
    }
}

#[test]
fn resolve_should_read_a_plain_value() {
    let set = one_file(vec![simple("life", "{0} to maximum Life")]);
    let r = resolve(&set, "  25 to   maximum Life ");
    assert_eq!(r.line, "25 to maximum Life");
    assert_eq!(values(&r), vec![("life".to_owned(), exact(25))]);
}

#[test]
fn resolve_should_read_signed_values_and_reject_signs_the_format_never_writes() {
    let set = one_file(vec![simple("life", "{0:+d} to maximum Life")]);
    assert_eq!(
        values(&resolve(&set, "+25 to maximum Life")),
        vec![("life".to_owned(), exact(25))]
    );
    assert_eq!(
        values(&resolve(&set, "-5 to maximum Life")),
        vec![("life".to_owned(), exact(-5))]
    );
    assert_eq!(
        rejections(&resolve(&set, "25 to maximum Life")),
        vec!["sign-mismatch"]
    );
    let plain = one_file(vec![simple("life", "{0} to maximum Life")]);
    assert_eq!(
        rejections(&resolve(&plain, "+25 to maximum Life")),
        vec!["sign-mismatch"]
    );
}

#[test]
fn resolve_should_read_ranges_as_intervals() {
    let set = one_file(vec![desc(
        &["min", "max"],
        vec![variant(&[ANY, ANY], "Adds {0} to {1} Fire Damage", &[])],
    )]);
    assert_eq!(
        values(&resolve(&set, "Adds (5-7) to (10-12) Fire Damage")),
        vec![
            ("min".to_owned(), ranged(5, 7)),
            ("max".to_owned(), ranged(10, 12))
        ]
    );
}

#[test]
fn resolve_should_read_signed_ranges_of_both_signs() {
    let set = one_file(vec![simple("s", "{0:+d} to X")]);
    assert_eq!(
        values(&resolve(&set, "+(5-10) to X")),
        vec![("s".to_owned(), ranged(5, 10))]
    );
    assert_eq!(
        values(&resolve(&set, "-(10-5) to X")),
        vec![("s".to_owned(), ranged(-10, -5))]
    );
    let plain = one_file(vec![simple("s", "{0} to X")]);
    assert_eq!(
        rejections(&resolve(&plain, "-(10-5) to X")),
        vec!["sign-mismatch"]
    );
}

#[test]
fn resolve_should_invert_negate_for_the_reduced_variant() {
    let set = one_file(vec![increased_reduced("s")]);
    assert_eq!(
        values(&resolve(&set, "10% reduced X")),
        vec![("s".to_owned(), exact(-10))]
    );
    assert_eq!(
        values(&resolve(&set, "10% increased X")),
        vec![("s".to_owned(), exact(10))]
    );
    assert_eq!(
        values(&resolve(&set, "(10-20)% reduced X")),
        vec![("s".to_owned(), ranged(-20, -10))]
    );
}

#[test]
fn resolve_should_fit_a_range_that_crosses_zero_to_the_variant_it_meets() {
    let set = one_file(vec![increased_reduced("s")]);
    assert_eq!(
        values(&resolve(&set, "(-10-10)% increased X")),
        vec![("s".to_owned(), ranged(-10, 10))]
    );
    // The reduced variant negates: shown -10..10 is raw 10..-10.
    assert_eq!(
        values(&resolve(&set, "(-10-10)% reduced X")),
        vec![("s".to_owned(), ranged(-10, 10))]
    );
}

#[test]
fn resolve_should_give_the_raw_interval_of_a_rounding_handler() {
    let set = one_file(vec![desc(
        &["regen_per_minute"],
        vec![variant(
            &[ANY],
            "Regenerate {0} Life per second",
            &[("per_minute_to_per_second", 0)],
        )],
    )]);
    assert_eq!(
        values(&resolve(&set, "Regenerate 1.7 Life per second")),
        vec![(
            "regen_per_minute".to_owned(),
            RawValue::Exact(StatValue { min: 99, max: 104 })
        )]
    );
    assert_eq!(
        rejections(&resolve(&set, "Regenerate 1.75 Life per second")),
        vec!["value-not-shown"]
    );
}

#[test]
fn resolve_should_narrow_an_exact_value_to_its_condition() {
    // %d of x / 100 shows 0 for -99..=99; the condition keeps 1..=99.
    let set = one_file(vec![desc(
        &["s"],
        vec![variant(
            &[POSITIVE],
            "{0} seconds",
            &[("divide_by_one_hundred_0dp", 0)],
        )],
    )]);
    assert_eq!(
        values(&resolve(&set, "0 seconds")),
        vec![(
            "s".to_owned(),
            RawValue::Exact(StatValue { min: 1, max: 99 })
        )]
    );
}

#[test]
fn resolve_should_reject_a_value_outside_the_condition() {
    let set = one_file(vec![desc(
        &["s"],
        vec![variant(&[POSITIVE], "{0}% increased X", &[])],
    )]);
    assert_eq!(
        rejections(&resolve(&set, "-5% increased X")),
        vec!["condition-fails"]
    );
    let not_one = one_file(vec![desc(
        &["s"],
        vec![variant(&[Condition::Not(1)], "{0} Projectiles", &[])],
    )]);
    assert_eq!(
        rejections(&resolve(&not_one, "1 Projectiles")),
        vec!["condition-fails"]
    );
}

#[test]
fn resolve_should_fill_stats_without_a_placeholder() {
    let set = one_file(vec![desc(
        &["a", "flag", "neg", "zero_ok"],
        vec![variant(
            &[ANY, POSITIVE, NEGATIVE, ZERO],
            "{0}% more X",
            &[],
        )],
    )]);
    let hidden = |condition, assumed| RawValue::Hidden { condition, assumed };
    assert_eq!(
        values(&resolve(&set, "5% more X")),
        vec![
            ("a".to_owned(), exact(5)),
            ("flag".to_owned(), hidden(POSITIVE, Some(1))),
            ("neg".to_owned(), hidden(NEGATIVE, Some(-1))),
            ("zero_ok".to_owned(), hidden(ZERO, Some(0))),
        ]
    );
    let not_zero = one_file(vec![desc(
        &["a"],
        vec![variant(&[Condition::Not(0)], "Cannot be Frozen", &[])],
    )]);
    assert_eq!(
        values(&resolve(&not_zero, "Cannot be Frozen")),
        vec![("a".to_owned(), hidden(Condition::Not(0), None))]
    );
    let impossible = Condition::Range {
        min: Some(3),
        max: Some(1),
    };
    let never = one_file(vec![desc(
        &["a"],
        vec![variant(&[impossible], "Never", &[])],
    )]);
    assert_eq!(
        rejections(&resolve(&never, "Never")),
        vec!["condition-fails"]
    );
}

#[test]
fn resolve_should_check_literal_numbers_of_the_template() {
    let set = one_file(vec![simple("s", "{0}% chance to cast Level 20 Spell")]);
    assert_eq!(
        values(&resolve(&set, "10% chance to cast Level 20 Spell")),
        vec![("s".to_owned(), exact(10))]
    );
    let r = resolve(&set, "10% chance to cast Level 18 Spell");
    match &r.outcome {
        Outcome::Unresolved(Unresolved::NoVariantFits(all)) => assert_eq!(
            all[0].reason,
            RejectReason::LiteralMismatch {
                expected: "20".to_owned(),
                found: "18".to_owned()
            }
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn resolve_should_require_a_sign_written_in_the_template() {
    let set = one_file(vec![simple("s", "+{0} to Level")]);
    assert_eq!(
        values(&resolve(&set, "+2 to Level")),
        vec![("s".to_owned(), exact(2))]
    );
    assert_eq!(
        values(&resolve(&set, "+(1-2) to Level")),
        vec![("s".to_owned(), ranged(1, 2))]
    );
    assert_eq!(
        rejections(&resolve(&set, "-2 to Level")),
        vec!["sign-mismatch"]
    );
}

#[test]
fn resolve_should_require_a_repeated_placeholder_to_agree() {
    let set = one_file(vec![simple("s", "{0} and again {0}")]);
    assert_eq!(
        values(&resolve(&set, "3 and again 3")),
        vec![("s".to_owned(), exact(3))]
    );
    assert_eq!(
        rejections(&resolve(&set, "3 and again 4")),
        vec!["inconsistent-values"]
    );
}

#[test]
fn resolve_should_match_template_text_without_markup() {
    let set = one_file(vec![simple(
        "s",
        "[Ref|Gain] {0} <enchanted>{{Endurance}} Charges",
    )]);
    assert_eq!(
        values(&resolve(&set, "Gain 2 Endurance Charges")),
        vec![("s".to_owned(), exact(2))]
    );
}

#[test]
fn resolve_should_report_ambiguity_between_descriptions_only() {
    // Two variants of one description share a template: not ambiguity.
    let shared = desc(
        &["s", "t"],
        vec![
            variant(&[POSITIVE, ZERO], "{0}% increased Damage", &[]),
            variant(&[POSITIVE, POSITIVE], "{0}% increased Damage", &[]),
        ],
    );
    let set = one_file(vec![shared.clone()]);
    assert_eq!(resolved(&resolve(&set, "5% increased Damage")).variant, 0);
    // Local and global stats with the same text: ambiguity.
    let set = one_file(vec![shared, simple("local", "{0}% increased Damage")]);
    match resolve(&set, "5% increased Damage").outcome {
        Outcome::Ambiguous(all) => {
            let at: Vec<_> = all.iter().map(|c| c.at.index).collect();
            assert_eq!(at, vec![0, 1]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn resolve_should_list_shadowed_descriptions_apart() {
    let set = set_of(vec![
        file("a.txt", &["b.txt"], vec![simple("s", "{0} own text")]),
        file(
            "b.txt",
            &[],
            vec![simple("s", "{0} base text"), simple("s", "{0} own text")],
        ),
    ]);
    // `s` is described in a.txt; both of b.txt's descriptions are shadowed.
    let only = resolve(&set, "5 base text");
    assert_eq!(only.outcome, Outcome::Unresolved(Unresolved::OnlyShadowed));
    assert_eq!(only.shadowed.len(), 1);
    assert_eq!(only.shadowed[0].at.depth, 1);
    // Same text in both files: the nearer one resolves, the other is listed.
    let both = resolve(&set, "5 own text");
    assert_eq!(resolved(&both).at.depth, 0);
    let shadowed: Vec<_> = both
        .shadowed
        .iter()
        .map(|c| (c.at.depth, c.at.index))
        .collect();
    assert_eq!(shadowed, vec![(1, 1)]);
}

#[test]
fn resolve_should_index_each_hand_of_a_handed_description() {
    let mut d = desc(
        &["main_min", "main_max"],
        vec![variant(&[ANY, ANY], "Adds {0} to {1} Damage", &[])],
    );
    d.off_hand_stats = Some(vec!["off_min".to_owned(), "off_max".to_owned()]);
    let set = one_file(vec![d]);
    match resolve(&set, "Adds 1 to 2 Damage").outcome {
        Outcome::Ambiguous(all) => {
            let hands: Vec<_> = all
                .iter()
                .map(|c| (c.at.hand, c.stats[1].stat.clone()))
                .collect();
            assert_eq!(
                hands,
                vec![
                    (Hand::Main, "main_max".to_owned()),
                    (Hand::Off, "off_max".to_owned())
                ]
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn resolve_should_reject_variants_it_cannot_invert() {
    let unknown = one_file(vec![desc(
        &["s"],
        vec![variant(&[ANY], "{0}% Leech", &[("old_leech_percent", 0)])],
    )]);
    assert_eq!(
        rejections(&resolve(&unknown, "1% Leech")),
        vec!["unknown-handler"]
    );
    let mut flagged = simple("s", "{0} Flagged");
    flagged.variants[0]
        .unknown_flags
        .push("new_flag".to_owned());
    assert_eq!(
        rejections(&resolve(&one_file(vec![flagged]), "1 Flagged")),
        vec!["unknown-flag"]
    );
    let mut malformed = simple("s", "{0} Malformed");
    malformed.variants[0].handlers.push(Handler::Malformed {
        name: "negate".to_owned(),
    });
    assert_eq!(
        rejections(&resolve(&one_file(vec![malformed]), "1 Malformed")),
        vec!["malformed-handler"]
    );
    let format = one_file(vec![simple("s", "{0:-d}% Wasting")]);
    assert_eq!(
        rejections(&resolve(&format, "1% Wasting")),
        vec!["unsupported-format"]
    );
}

#[test]
fn resolve_should_report_a_line_no_template_has() {
    let set = one_file(vec![simple("s", "{0} to X")]);
    let r = resolve(&set, "Something else");
    assert_eq!(r.outcome, Outcome::Unresolved(Unresolved::NoTemplate));
    assert!(resolve(&set, "").outcome == Outcome::Unresolved(Unresolved::NoTemplate));
}

#[test]
fn build_should_count_every_variant_it_leaves_out() {
    let mut quality = simple("q", "{0} quality");
    quality.variants[0].gem_quality = true;
    let mut table = simple("t", "{0} table");
    table.variants[0].table_only = true;
    let lookup = desc(
        &["p"],
        vec![variant(&[ANY], "Allocates {0}", &[("passive_hash", 0)])],
    );
    // A parsed template whose placeholder is beyond the stat count (the
    // loader rejects this; built by hand to check the index does too).
    let mut out_of_range = simple("oor", "{0} beyond");
    out_of_range.variants[0].template = Ok(Template {
        segments: vec![
            Segment::Value {
                index: 3,
                format: PlaceholderFormat::Plain,
                written: String::new(),
            },
            Segment::Text(" beyond".to_owned()),
        ],
    });
    let set = one_file(vec![
        quality,
        table,
        simple("bad", "{0"),
        out_of_range,
        simple("empty", "<<IconOnly>>"),
        simple("multi", "{0} first\\nsecond"),
        lookup,
        simple("ok", "{0} fine"),
        simple("ok2", "{0} fine"),
    ]);
    let scope = set.scope("a.txt").unwrap();
    let stats = Index::build(&scope).stats();
    assert_eq!(
        stats,
        IndexStats {
            indexed: 3,
            multi_line: 1,
            lookup: 0,
            entries: 3,
            keys: 2,
            max_lines: 2,
            gem_quality: 1,
            table_only: 1,
            bad_template: 2,
            empty_text: 1,
            lookup_unavailable: 1,
            lookup_multi: 0,
            lookup_blank_names: 0,
        }
    );
    // Every variant is indexed or counted as left out.
    let accounted = stats.indexed
        + stats.gem_quality
        + stats.table_only
        + stats.bad_template
        + stats.empty_text
        + stats.lookup_unavailable
        + stats.lookup_multi;
    assert_eq!(accounted, 9);
}

#[test]
fn resolve_should_keep_a_later_variant_that_reads_the_line_differently() {
    // Same text for positive values and for negated negative values; a
    // third variant agrees with the first and is not listed.
    let set = one_file(vec![desc(
        &["s"],
        vec![
            variant(&[POSITIVE], "{0}% chance", &[]),
            variant(&[NEGATIVE], "{0}% chance", &[("negate", 0)]),
            variant(&[ANY], "{0}% chance", &[]),
        ],
    )]);
    let r = resolve(&set, "5% chance");
    let c = resolved(&r);
    assert_eq!((c.variant, c.stats[0].value.clone()), (0, exact(5)));
    assert_eq!(
        c.alternatives,
        vec![Alternative {
            variant: 1,
            stats: vec![ResolvedStat {
                stat: "s".to_owned(),
                value: exact(-5)
            }]
        }]
    );
}

#[test]
fn build_should_not_shadow_the_off_hand_of_a_stat_in_both_hands() {
    let mut d = desc(
        &["x", "main_only"],
        vec![variant(&[ANY, ANY], "{0} and {1} Hands", &[])],
    );
    d.off_hand_stats = Some(vec!["x".to_owned(), "off_only".to_owned()]);
    let set = one_file(vec![d]);
    let r = resolve(&set, "1 and 2 Hands");
    assert!(r.shadowed.is_empty(), "{:?}", r.shadowed);
    assert!(matches!(r.outcome, Outcome::Ambiguous(ref all) if all.len() == 2));
}

/// Deterministic generator (xorshift64); no external crate.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap()).unwrap()
    }

    fn value(&mut self, lo: i64, hi: i64) -> i64 {
        lo + i64::try_from(self.next() % u64::try_from(hi - lo + 1).unwrap()).unwrap()
    }
}

/// Property: what the renderer writes, the resolver reads back to the same
/// description with intervals that hold the rendered values.
#[test]
fn resolve_should_read_back_what_the_renderer_writes() {
    let handlers = [
        None,
        Some("negate"),
        Some("divide_by_one_hundred"),
        Some("per_minute_to_per_second"),
        Some("milliseconds_to_seconds_2dp_if_required"),
        Some("divide_by_ten_0dp"),
        Some("deciseconds_to_seconds"),
        Some("invert_chance"),
        Some("multiplicative_damage_modifier"),
    ];
    let texts = ["{0} X {1}", "{0:+d}% X {1:+d} Y", "Y {1} and {0}% Z"];
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..3_000 {
        let text = texts[rng.below(texts.len())];
        let h: Vec<(&str, usize)> = (0..2)
            .filter_map(|stat| handlers[rng.below(handlers.len())].map(|h| (h, stat)))
            .collect();
        let d = desc(&["a", "b"], vec![variant(&[ANY, ANY], text, &h)]);
        let raw: Vec<StatValue> = (0..2)
            .map(|_| {
                let a = rng.value(-5_000, 5_000);
                if rng.below(3) == 0 {
                    let b = rng.value(a, a + 500);
                    StatValue { min: a, max: b }
                } else {
                    StatValue::exact(a)
                }
            })
            .collect();
        // Conditions are open, so every case renders.
        let (_, lines) =
            render_description(&d, Hand::Main, &raw, &NoLookup, VariantFilter::default()).unwrap();
        let set = one_file(vec![d]);
        let r = resolve(&set, &lines[0]);
        let c = match &r.outcome {
            Outcome::Resolved(c) => c,
            other => panic!("{text} {h:?} {raw:?} -> {:?}: {other:?}", lines[0]),
        };
        for (stat, raw) in c.stats.iter().zip(&raw) {
            let (RawValue::Exact(got) | RawValue::Ranged(got)) = stat.value else {
                panic!("hidden {stat:?}");
            };
            assert!(
                got.min <= raw.min && raw.max <= got.max,
                "{text} {h:?} {raw:?} -> {:?}: {got:?}",
                lines[0]
            );
        }
    }
}

/// Property: with conditions that pick the variant (increased for positive
/// raw minimums, reduced and negated for negative ones), the variant the
/// renderer chose reads the line back, and its value holds the raw one.
#[test]
fn resolve_should_read_back_the_variant_the_conditions_chose() {
    let set = one_file(vec![increased_reduced("s")]);
    let d = increased_reduced("s");
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    let mut rendered = 0;
    for _ in 0..3_000 {
        let a = rng.value(-300, 300);
        let raw = if rng.below(2) == 0 {
            StatValue::exact(a)
        } else {
            StatValue {
                min: a,
                max: rng.value(a, a + 200),
            }
        };
        // Raw minimum 0 meets neither condition; nothing to read back.
        let Ok((variant, lines)) =
            render_description(&d, Hand::Main, &[raw], &NoLookup, VariantFilter::default())
        else {
            assert_eq!(raw.min, 0);
            continue;
        };
        rendered += 1;
        let r = resolve(&set, &lines[0]);
        let c = resolved(&r);
        assert_eq!(c.variant, variant, "{raw:?} -> {}", lines[0]);
        let (RawValue::Exact(got) | RawValue::Ranged(got)) = c.stats[0].value else {
            panic!("hidden");
        };
        assert!(
            got.min <= raw.min && raw.max <= got.max,
            "{raw:?} -> {}: {got:?}",
            lines[0]
        );
    }
    assert!(rendered > 2_900, "{rendered}");
}

/// Property: resolve never panics, whatever the text.
#[test]
fn resolve_should_not_panic_on_arbitrary_text() {
    let set = one_file(vec![
        increased_reduced("s"),
        simple("t", "{0:+d} to {0} Level 20"),
        simple("u", "+{0}"),
        desc(
            &["a", "b"],
            vec![variant(
                &[ANY, Condition::Not(0)],
                "Adds {0} to {1}",
                &[("divide_by_ten_1dp", 1)],
            )],
        ),
    ]);
    let scope = set.scope("a.txt").unwrap();
    let index = Index::build(&scope);
    let pieces = [
        "+",
        "-",
        "(",
        ")",
        "0",
        "5",
        "10",
        ".",
        "%",
        " ",
        "\t",
        "increased",
        "reduced",
        "X",
        "to",
        "Level",
        "Adds",
        "\u{E000}",
        "é",
        "--",
        "(-",
        "-)",
        "99999999999999999999",
        "1.5",
        "{0}",
        "\\n",
    ];
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    let mut outcomes = BTreeMap::<&str, usize>::new();
    for _ in 0..20_000 {
        let n = rng.below(10);
        let line: String = (0..n).map(|_| pieces[rng.below(pieces.len())]).collect();
        let name = match index.resolve(&line).outcome {
            Outcome::Resolved(_) => "resolved",
            Outcome::Ambiguous(_) => "ambiguous",
            Outcome::Unresolved(u) => u.name(),
        };
        *outcomes.entry(name).or_default() += 1;
    }
    // The generator reaches more than the no-template path.
    assert!(outcomes.len() >= 3, "{outcomes:?}");
}

/// Names for `passive_hash` (keys 7 and 9 share a name, 10 is blank),
/// `display_indexable_support` (a name with a number), and
/// `tree_expansion_jewel_passive` (a two-line name, and its first line
/// alone as another key).
struct Names;

impl Lookup for Names {
    fn name(&self, handler: LookupHandler, value: i64) -> Option<String> {
        self.entries(handler)
            .into_iter()
            .find(|(k, _)| *k == value)
            .map(|(_, n)| n)
    }

    fn entries(&self, handler: LookupHandler) -> Vec<(i64, String)> {
        match handler {
            LookupHandler::PassiveHash => vec![
                (7, "Whispers of Doom".to_owned()),
                (8, "Heart of Ice".to_owned()),
                (9, "Whispers of Doom".to_owned()),
                (10, "  ".to_owned()),
            ],
            LookupHandler::DisplayIndexableSupport => vec![(1, "Level 2 Thing".to_owned())],
            LookupHandler::TreeExpansionJewelPassive => vec![
                (3, "Grant: 12% Fire\n  Grant: 8%  Cold ".to_owned()),
                (4, "Grant: 12% Fire".to_owned()),
            ],
            _ => Vec::new(),
        }
    }
}

fn resolve_named(set: &StatDescriptions, line: &str) -> Resolution {
    let scope = set.scope("a.txt").unwrap();
    Index::build_with(&scope, &Names).resolve(line)
}

fn outcome_name(r: &Resolution) -> &'static str {
    match &r.outcome {
        Outcome::Resolved(_) => "resolved",
        Outcome::Ambiguous(_) => "ambiguous",
        Outcome::Unresolved(u) => u.name(),
    }
}

#[test]
fn resolve_lines_should_read_a_looked_up_name_that_spans_lines() {
    let set = one_file(vec![desc(
        &["n"],
        vec![variant(
            &[ANY],
            "{0}",
            &[("tree_expansion_jewel_passive", 0)],
        )],
    )]);
    let scope = set.scope("a.txt").unwrap();
    let index = Index::build_with(&scope, &Names);
    let two = index.resolve_lines(&["Grant: 12% Fire", "Grant: 8% Cold"]);
    assert_eq!(two.len(), 1);
    assert_eq!(
        (two[0].lines, values(&two[0])),
        (2, vec![("n".to_owned(), RawValue::Lookup(vec![3]))])
    );
    let one = index.resolve_lines(&["Grant: 12% Fire"]);
    assert_eq!(
        values(&one[0]),
        vec![("n".to_owned(), RawValue::Lookup(vec![4]))]
    );
}

#[test]
fn resolve_should_read_a_looked_up_name_as_every_key_with_that_name() {
    let set = one_file(vec![desc(
        &["p"],
        vec![variant(&[ANY], "Allocates {0}", &[("passive_hash", 0)])],
    )]);
    let r = resolve_named(&set, "Allocates Whispers of Doom");
    assert_eq!(
        values(&r),
        vec![("p".to_owned(), RawValue::Lookup(vec![7, 9]))]
    );
    let r = resolve_named(&set, "Allocates  Heart of Ice ");
    assert_eq!(
        values(&r),
        vec![("p".to_owned(), RawValue::Lookup(vec![8]))]
    );
    assert_eq!(
        resolve_named(&set, "Allocates Nothing").outcome,
        Outcome::Unresolved(Unresolved::NoTemplate)
    );
    let scope = set.scope("a.txt").unwrap();
    let stats = Index::build_with(&scope, &Names).stats();
    // Two distinct non-blank names; the blank one is counted.
    assert_eq!(
        (
            stats.indexed,
            stats.lookup,
            stats.entries,
            stats.lookup_blank_names
        ),
        (1, 1, 2, 1)
    );
}

#[test]
fn build_should_count_two_lookup_handlers_on_one_stat_as_multi() {
    let set = one_file(vec![desc(
        &["p"],
        vec![variant(
            &[ANY],
            "Allocates {0}",
            &[("passive_hash", 0), ("display_indexable_support", 0)],
        )],
    )]);
    let scope = set.scope("a.txt").unwrap();
    let stats = Index::build_with(&scope, &Names).stats();
    assert_eq!((stats.indexed, stats.lookup_multi), (0, 1));
}

#[test]
fn resolve_lines_should_keep_single_line_readings_when_a_multi_line_match_fails() {
    // The two-line key fits, its condition does not; both lines resolve
    // alone, so their single-line readings are kept.
    let set = one_file(vec![
        desc(
            &["a"],
            vec![variant(&[POSITIVE], "{0} first\\nsecond", &[])],
        ),
        simple("one", "{0} first"),
        simple("two", "second"),
    ]);
    let scope = set.scope("a.txt").unwrap();
    let index = Index::build(&scope);
    let rs = index.resolve_lines(&["-3 first", "second"]);
    let shapes: Vec<(usize, &str)> = rs.iter().map(|r| (r.lines, outcome_name(r))).collect();
    assert_eq!(shapes, vec![(1, "resolved"), (1, "resolved")]);
    assert_eq!(values(&rs[0]), vec![("one".to_owned(), exact(-3))]);
}

#[test]
fn resolve_should_keep_only_looked_up_keys_that_meet_the_condition() {
    let at_most_eight = Condition::Range {
        min: None,
        max: Some(8),
    };
    let set = one_file(vec![desc(
        &["p"],
        vec![variant(
            &[at_most_eight],
            "Allocates {0}",
            &[("passive_hash", 0)],
        )],
    )]);
    let r = resolve_named(&set, "Allocates Whispers of Doom");
    assert_eq!(
        values(&r),
        vec![("p".to_owned(), RawValue::Lookup(vec![7]))]
    );
    let above = Condition::Range {
        min: Some(10),
        max: None,
    };
    let set = one_file(vec![desc(
        &["p"],
        vec![variant(&[above], "Allocates {0}", &[("passive_hash", 0)])],
    )]);
    assert_eq!(
        rejections(&resolve_named(&set, "Allocates Heart of Ice")),
        vec!["condition-fails"]
    );
}

#[test]
fn resolve_should_read_numbers_next_to_and_inside_a_looked_up_name() {
    let set = one_file(vec![desc(
        &["level", "gem"],
        vec![variant(
            &[ANY, ANY],
            "Supported by Level {0} {1}",
            &[("display_indexable_support", 1)],
        )],
    )]);
    let r = resolve_named(&set, "Supported by Level 20 Level 2 Thing");
    assert_eq!(
        values(&r),
        vec![
            ("level".to_owned(), exact(20)),
            ("gem".to_owned(), RawValue::Lookup(vec![1]))
        ]
    );
    // The name's own number is a literal.
    assert_eq!(
        rejections(&resolve_named(&set, "Supported by Level 20 Level 3 Thing")),
        vec!["literal-mismatch"]
    );
}

#[test]
fn build_should_count_lookup_variants_it_cannot_expand() {
    let two = desc(
        &["a", "b"],
        vec![variant(
            &[ANY, ANY],
            "{0} and {1}",
            &[("passive_hash", 0), ("passive_hash", 1)],
        )],
    );
    let missing = desc(
        &["c"],
        vec![variant(
            &[ANY],
            "Uses {0}",
            &[("affliction_reward_type", 0)],
        )],
    );
    // A lookup stat without a placeholder is an ordinary hidden stat.
    let hidden = desc(
        &["d", "e"],
        vec![variant(&[ANY, ANY], "{0} hidden", &[("passive_hash", 1)])],
    );
    let set = one_file(vec![two, missing, hidden]);
    let scope = set.scope("a.txt").unwrap();
    let index = Index::build_with(&scope, &Names);
    let stats = index.stats();
    assert_eq!(
        (
            stats.indexed,
            stats.lookup,
            stats.lookup_multi,
            stats.lookup_unavailable
        ),
        (1, 0, 1, 1)
    );
    assert_eq!(outcome_name(&index.resolve("5 hidden")), "resolved");
}

#[test]
fn resolve_lines_should_match_a_multi_line_template_across_lines() {
    let set = one_file(vec![
        desc(
            &["a", "b"],
            vec![variant(&[ANY, ANY], "{0} first\\n<tag>{{Second}} {1}", &[])],
        ),
        simple("one", "{0} first"),
    ]);
    let scope = set.scope("a.txt").unwrap();
    let index = Index::build(&scope);
    assert_eq!(index.stats().max_lines, 2);
    let rs = index.resolve_lines(&["3 first", " Second  4", "5 first"]);
    assert_eq!(rs.len(), 2);
    assert_eq!((rs[0].lines, rs[0].line.as_str()), (2, "3 first\nSecond 4"));
    assert_eq!(
        values(&rs[0]),
        vec![("a".to_owned(), exact(3)), ("b".to_owned(), exact(4))]
    );
    assert_eq!(rs[1].lines, 1);
    assert_eq!(values(&rs[1]), vec![("one".to_owned(), exact(5))]);
    // Alone, the first line is the single-line description.
    assert_eq!(
        values(&index.resolve("3 first")),
        vec![("one".to_owned(), exact(3))]
    );
}

#[test]
fn resolve_lines_should_keep_a_multi_line_key_match_that_does_not_verify() {
    let set = one_file(vec![
        desc(
            &["a"],
            vec![variant(&[POSITIVE], "{0} first\\nsecond", &[])],
        ),
        simple("one", "{0} first"),
    ]);
    let scope = set.scope("a.txt").unwrap();
    let index = Index::build(&scope);
    let rs = index.resolve_lines(&["-3 first", "second"]);
    assert_eq!(rs.len(), 1);
    assert_eq!(rejections(&rs[0]), vec!["condition-fails"]);
}

#[test]
fn resolve_lines_should_cover_every_line_once() {
    let set = one_file(vec![
        desc(&["a"], vec![variant(&[ANY], "A {0}\\nB\\nC", &[])]),
        simple("b", "B"),
    ]);
    let scope = set.scope("a.txt").unwrap();
    let index = Index::build(&scope);
    let lines = ["A 1", "B", "x", "", "A 2", "B", "C", "B"];
    let rs = index.resolve_lines(&lines);
    let covered: usize = rs.iter().map(|r| r.lines).sum();
    assert_eq!(covered, lines.len());
    let shapes: Vec<(usize, &str)> = rs.iter().map(|r| (r.lines, outcome_name(r))).collect();
    assert_eq!(
        shapes,
        vec![
            (1, "no-template"),
            (1, "resolved"),
            (1, "no-template"),
            (1, "no-template"),
            (3, "resolved"),
            (1, "resolved"),
        ]
    );
    assert!(index.resolve_lines(&[]).is_empty());
}

#[test]
fn resolve_lines_should_read_back_multi_line_and_lookup_renders() {
    let set = one_file(vec![
        desc(
            &["lvl", "gem"],
            vec![variant(
                &[ANY, ANY],
                "Level {0}\\nSupported by {1}",
                &[("display_indexable_support", 1)],
            )],
        ),
        desc(
            &["p", "q"],
            vec![variant(
                &[ANY, POSITIVE],
                "Allocates {0}\\n{1}% increased X\\nThird line",
                &[("passive_hash", 0)],
            )],
        ),
    ]);
    let scope = set.scope("a.txt").unwrap();
    let index = Index::build_with(&scope, &Names);
    let (_, one) = scope.description(scope.find("lvl").unwrap()).unwrap();
    let (_, two) = scope.description(scope.find("p").unwrap()).unwrap();
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    for _ in 0..500 {
        let level = i64::try_from(rng.below(40)).unwrap() - 5;
        let key = [7, 8, 9][rng.below(3)];
        let pct = i64::try_from(rng.below(100)).unwrap() + 1;
        let (_, a) = render_description(
            one,
            Hand::Main,
            &[StatValue::exact(level), StatValue::exact(1)],
            &Names,
            VariantFilter::default(),
        )
        .unwrap();
        let (_, b) = render_description(
            two,
            Hand::Main,
            &[StatValue::exact(key), StatValue::exact(pct)],
            &Names,
            VariantFilter::default(),
        )
        .unwrap();
        let lines: Vec<&str> = a.iter().chain(&b).map(String::as_str).collect();
        let rs = index.resolve_lines(&lines);
        assert_eq!(rs.len(), 2, "{lines:?}");
        assert_eq!(
            values(&rs[0]),
            vec![
                ("lvl".to_owned(), exact(level)),
                ("gem".to_owned(), RawValue::Lookup(vec![1]))
            ]
        );
        let keys = if key == 8 { vec![8] } else { vec![7, 9] };
        assert_eq!(
            values(&rs[1]),
            vec![
                ("p".to_owned(), RawValue::Lookup(keys)),
                ("q".to_owned(), exact(pct))
            ]
        );
    }
}
