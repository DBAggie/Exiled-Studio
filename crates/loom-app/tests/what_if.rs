//! What-if queries (`loom_app::what_if`) on the synthetic PoE1 bundle and
//! build: a series across the stages, a gem's level, the character's
//! level, and a config number; each point equal to what the real command
//! and a calculation give; nothing committed; bad requests refused.

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]
#![expect(clippy::panic, reason = "tests; a failure should panic")]

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use loom_app::{App, Config, HostCommand, Session, api};
use loom_persistence::Store;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bundle_dir() -> PathBuf {
    root().join("crates/loom-data/tests/fixtures/bundle-poe1")
}

fn synthetic() -> String {
    std::fs::read_to_string(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
    )
    .unwrap()
}

/// The synthetic build with two stages (PoB's loadouts), as
/// `workflow.rs` makes it: two specs, two skill sets, and two item sets
/// linked by `{1}` and `{2}`.
fn staged() -> String {
    let skill = |name: &str, level: u32| {
        format!(
            "<Skill enabled=\"true\" slot=\"Weapon 1\" mainActiveSkill=\"1\">\
             <Gem nameSpec=\"{name}\" skillId=\"{name}\" level=\"{level}\" quality=\"0\" \
             enabled=\"true\"/></Skill>"
        )
    };
    let xml = synthetic()
        .replace(
            "<Spec treeVersion=\"3_29\" classId=\"0\" ascendClassId=\"0\" nodes=\"\"/>",
            "<Spec title=\"^4Leveling {1}\" treeVersion=\"3_29\" classId=\"0\" ascendClassId=\"0\" nodes=\"\"/>\
             <Spec title=\"Endgame {2}\" treeVersion=\"3_29\" classId=\"0\" ascendClassId=\"0\" nodes=\"\"/>",
        )
        .replace(
            "<Skills/>",
            &format!(
                "<Skills activeSkillSet=\"1\"><SkillSet id=\"1\" title=\"Leveling {{1}}\">{}{}</SkillSet>\
                 <SkillSet id=\"2\" title=\"Endgame {{2}}\">{}</SkillSet></Skills>",
                skill("Smite", 1),
                skill("Leap Slam", 1),
                skill("Smite", 20),
            ),
        )
        .replace("<ItemSet id=\"1\" ", "<ItemSet id=\"1\" title=\"Leveling {1}\" ")
        .replace(
            "</ItemSet>",
            "</ItemSet><ItemSet id=\"2\" title=\"Endgame {2}\" useSecondWeaponSet=\"false\">\
             <Slot name=\"Ring 1\" itemId=\"0\"/></ItemSet>",
        );
    assert!(
        xml.contains("SkillSet id=\"2\""),
        "the stages were not added"
    );
    xml
}

fn session(xml: &str) -> Session {
    let mut s = Session::new(Store::new());
    s.set_data(loom_app::GameData::load(&bundle_dir()));
    s.import_text(xml, None).unwrap();
    s
}

fn idle() -> api::CalcStatus {
    api::CalcStatus {
        state: api::CalcState::Idle,
        version: 0,
        result_state: None,
        error: None,
    }
}

/// The session's own calculation of its build, as a point's fields.
fn calculated(s: &Session) -> Vec<api::WhatIfField> {
    let result = loom_app::calculate(s.state().unwrap(), s.data().unwrap(), &|| false).unwrap();
    result
        .fields
        .iter()
        .map(|(field, f)| api::WhatIfField {
            field: field.to_string(),
            value: f.value.filter(|v| v.is_finite()),
            state: f.state.kind().name().to_owned(),
        })
        .collect()
}

fn ask(s: &Session, vary: api::WhatIfVary) -> Result<api::WhatIfSeries, loom_app::AppError> {
    loom_app::what_if(
        s.state().unwrap(),
        s.data().unwrap(),
        &api::WhatIfRequest { vary },
        &|| false,
    )
}

/// `(label, x, current)` of each point.
fn shape(series: &api::WhatIfSeries) -> Vec<(&str, f64, bool)> {
    series
        .points
        .iter()
        .map(|p| (p.label.as_str(), p.x, p.current))
        .collect()
}

/// The fields whose value or state differs between two points.
fn differing(a: &[api::WhatIfField], b: &[api::WhatIfField]) -> Vec<String> {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .filter(|(x, y)| x != y)
        .map(|(x, _)| x.field.clone())
        .collect()
}

#[test]
fn what_if_stages_should_give_one_point_per_loadout_and_leave_the_build_unchanged() {
    let mut s = session(&staged());
    let version = s.version();
    let before = Arc::clone(s.state().unwrap());

    let series = ask(&s, api::WhatIfVary::Stages).unwrap();
    assert_eq!(
        shape(&series),
        [("Leveling", 0.0, true), ("Endgame", 1.0, false)]
    );
    assert!(!series.points[0].fields.is_empty());
    assert_eq!(series.points[0].fields, calculated(&s));

    // Nothing was committed: the same state, version, and history.
    assert!(Arc::ptr_eq(&before, s.state().unwrap()));
    assert_eq!(s.version(), version);
    let history = s.snapshot(idle()).history;
    assert!(history.undo.is_none() && history.redo.is_none());

    // The other stage's point is what choosing the stage calculates.
    s.use_loadout(2).unwrap().unwrap();
    assert_eq!(series.points[1].fields, calculated(&s));
    // And asked from there, the second stage is the current one.
    let series = ask(&s, api::WhatIfVary::Stages).unwrap();
    assert_eq!(
        shape(&series),
        [("Leveling", 0.0, false), ("Endgame", 1.0, true)]
    );
}

#[test]
fn what_if_gem_level_should_match_the_real_edit() {
    let mut s = session(&staged());
    let series = ask(
        &s,
        api::WhatIfVary::GemLevel {
            group: 0,
            gem: 0,
            from: 1,
            to: 5,
        },
    )
    .unwrap();
    assert_eq!(
        shape(&series),
        [
            ("1", 1.0, true),
            ("2", 2.0, false),
            ("3", 3.0, false),
            ("4", 4.0, false),
            ("5", 5.0, false),
        ]
    );
    assert_eq!(series.points[0].fields, calculated(&s));

    s.edit_gem(0, 0, Some(4), None, None).unwrap().unwrap();
    assert_eq!(series.points[3].fields, calculated(&s));
    // From the edited build, level 4 is the current point.
    let series = ask(
        &s,
        api::WhatIfVary::GemLevel {
            group: 0,
            gem: 0,
            from: 4,
            to: 4,
        },
    )
    .unwrap();
    assert_eq!(shape(&series), [("4", 4.0, true)]);
    assert_eq!(series.points[0].fields, calculated(&s));
}

#[test]
fn what_if_character_level_should_give_a_point_per_step_as_a_build_of_that_level() {
    let s = session(&synthetic());
    let series = ask(
        &s,
        api::WhatIfVary::CharacterLevel {
            from: 1,
            to: 90,
            step: 40,
        },
    )
    .unwrap();
    // 1, 41, 81: the last step ends at or below `to`.
    assert_eq!(
        shape(&series),
        [("1", 1.0, true), ("41", 41.0, false), ("81", 81.0, false)]
    );
    assert_eq!(series.points[0].fields, calculated(&s));
    assert!(s.snapshot(idle()).history.undo.is_none());
    // No command sets the level, so the point at 81 is compared with the
    // same build imported at level 81. (The synthetic bundle has no life
    // or mana per level, so no field's number moves with the level here.)
    let at_81 = session(&synthetic().replace("<Build level=\"1\"", "<Build level=\"81\""));
    assert_eq!(
        at_81.snapshot(idle()).build.unwrap().level,
        Some(81),
        "the fixture's level was not replaced"
    );
    assert_eq!(series.points[2].fields, calculated(&at_81));
    assert_eq!(
        shape(
            &ask(
                &at_81,
                api::WhatIfVary::CharacterLevel {
                    from: 80,
                    to: 82,
                    step: 1,
                },
            )
            .unwrap()
        ),
        [("80", 80.0, false), ("81", 81.0, true), ("82", 82.0, false)]
    );
}

#[test]
fn what_if_config_number_should_match_the_real_input() {
    use api::ConfigValueView;
    let mut s = session(&synthetic());
    s.set_config_input("resistancePenalty", &ConfigValueView::Number(-60.0))
        .unwrap()
        .unwrap();
    let version = s.version();
    let vary = || api::WhatIfVary::ConfigNumber {
        name: "resistancePenalty".to_owned(),
        from: -60.0,
        to: 0.0,
        steps: 3,
    };
    let series = ask(&s, vary()).unwrap();
    assert_eq!(
        shape(&series),
        [
            ("-60", -60.0, true),
            ("-30", -30.0, false),
            ("0", 0.0, false)
        ]
    );
    assert_eq!(series.points[0].fields, calculated(&s));
    assert_eq!(s.version(), version);
    // The swept value reaches the calculation: the resistances move.
    let moved = differing(&series.points[0].fields, &series.points[2].fields);
    assert!(
        moved.iter().any(|f| f == "resistance.fire"),
        "resistance.fire did not change: {moved:?}"
    );

    s.set_config_input("resistancePenalty", &ConfigValueView::Number(-30.0))
        .unwrap()
        .unwrap();
    assert_eq!(series.points[1].fields, calculated(&s));
    assert_eq!(
        shape(&ask(&s, vary()).unwrap()),
        [
            ("-60", -60.0, false),
            ("-30", -30.0, true),
            ("0", 0.0, false)
        ]
    );
}

#[test]
fn what_if_should_refuse_bad_ranges() {
    use api::WhatIfVary::{CharacterLevel, ConfigNumber, GemLevel};
    let mut s = session(&staged());
    s.set_config_input("resistancePenalty", &api::ConfigValueView::Number(-60.0))
        .unwrap();
    s.set_config_input("conditionFullLife", &api::ConfigValueView::Boolean(true))
        .unwrap();
    let gem = |group, gem, from, to| GemLevel {
        group,
        gem,
        from,
        to,
    };
    let config = |name: &str, from, to, steps| ConfigNumber {
        name: name.to_owned(),
        from,
        to,
        steps,
    };
    let refused = [
        // Too many points (41 is the most).
        gem(0, 0, 1, 42),
        CharacterLevel {
            from: 1,
            to: 100,
            step: 1,
        },
        config("resistancePenalty", -60.0, 0.0, 42),
        // Ranges that are not ranges.
        gem(0, 0, 5, 4),
        gem(0, 0, 0, 3),
        CharacterLevel {
            from: 50,
            to: 40,
            step: 1,
        },
        CharacterLevel {
            from: 1,
            to: 40,
            step: 0,
        },
        CharacterLevel {
            from: 0,
            to: 40,
            step: 1,
        },
        CharacterLevel {
            from: 90,
            to: 101,
            step: 1,
        },
        config("resistancePenalty", 0.0, -60.0, 3),
        config("resistancePenalty", 0.0, 0.0, 3),
        config("resistancePenalty", f64::NAN, 0.0, 3),
        config("resistancePenalty", -60.0, f64::INFINITY, 3),
        config("resistancePenalty", -60.0, 0.0, 1),
        // A group, gem, or numeric input the build does not have.
        gem(9, 0, 1, 2),
        gem(0, 5, 1, 2),
        config("enemyLevel", 1.0, 84.0, 3),
        config("conditionFullLife", 0.0, 1.0, 2),
    ];
    for vary in refused {
        let text = format!("{vary:?}");
        match ask(&s, vary) {
            Err(e) => assert_eq!(e.code, "APP_WHAT_IF", "{text}: {e}"),
            Ok(_) => panic!("{text} was not refused"),
        }
    }
    // The limits themselves pass.
    assert_eq!(ask(&s, gem(0, 0, 1, 41)).unwrap().points.len(), 41);
    assert_eq!(
        ask(&s, config("resistancePenalty", -60.0, 0.0, 2))
            .unwrap()
            .points
            .len(),
        2
    );
}

#[test]
fn what_if_should_stop_with_the_calculations_error_when_cancelled() {
    let s = session(&staged());
    let cancelled = loom_app::what_if(
        s.state().unwrap(),
        s.data().unwrap(),
        &api::WhatIfRequest {
            vary: api::WhatIfVary::Stages,
        },
        &|| true,
    )
    .unwrap_err();
    let own = loom_app::calculate(s.state().unwrap(), s.data().unwrap(), &|| true).unwrap_err();
    assert_eq!(cancelled.code, "CALC_CANCELLED");
    assert_eq!(cancelled, own);
}

#[test]
fn what_if_should_say_when_the_data_cannot_calculate_the_build() {
    let s = session(&staged());
    let other =
        loom_app::GameData::load(&root().join("crates/loom-data/tests/fixtures/bundle-poe2"))
            .unwrap();
    let refused = loom_app::what_if(
        s.state().unwrap(),
        &other,
        &api::WhatIfRequest {
            vary: api::WhatIfVary::Stages,
        },
        &|| false,
    )
    .unwrap_err();
    assert_eq!(refused.code, "APP_GAME_MISMATCH");
}

/// The running application answers on the caller's thread and commits
/// nothing: the version and the history stay.
#[test]
fn the_app_should_answer_a_what_if_query_without_changing_the_build() {
    let dir = std::env::temp_dir().join(format!("loom-app-what-if-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    let sink: loom_app::Sink = Arc::new(move |e| {
        let _ = tx.lock().unwrap().send(e);
    });
    let mut config = Config::new(dir.clone(), Some(bundle_dir())).with_test_keys(true);
    config.instance = "what-if".to_owned();
    let app = App::start(config, sink).unwrap();
    let request = api::WhatIfRequest {
        vary: api::WhatIfVary::Stages,
    };
    assert_eq!(app.what_if(&request).unwrap_err().code, "APP_NO_BUILD");

    let outcome = app
        .execute(HostCommand::Api(api::Command::ImportText {
            text: staged(),
        }))
        .unwrap();
    let api::Outcome::Done { version } = outcome else {
        panic!("the import gave {outcome:?}");
    };
    loop {
        let e: api::Event = events.recv_timeout(Duration::from_secs(30)).unwrap();
        if e.version == version && matches!(e.kind, api::EventKind::CalcFinished { .. }) {
            break;
        }
    }

    let series = app.what_if(&request).unwrap();
    assert_eq!(
        shape(&series),
        [("Leveling", 0.0, true), ("Endgame", 1.0, false)]
    );
    // The current point is the application's own result, field by field.
    let (_, document) = app.calc_document().unwrap();
    let document: serde_json::Value = serde_json::from_str(&document.to_string()).unwrap();
    let fields = document["result"]["fields"].as_array().unwrap();
    assert_eq!(fields.len(), series.points[0].fields.len());
    for (theirs, ours) in fields.iter().zip(&series.points[0].fields) {
        assert_eq!(theirs["field"].as_str(), Some(ours.field.as_str()));
        assert_eq!(theirs["state"].as_str(), Some(ours.state.as_str()));
        assert_eq!(theirs["value"].as_f64(), ours.value, "{}", ours.field);
    }
    let snapshot = app.snapshot();
    assert_eq!(snapshot.version, version);
    assert!(snapshot.history.undo.is_none());
    drop(app);
    let _ = std::fs::remove_dir_all(&dir);
}
