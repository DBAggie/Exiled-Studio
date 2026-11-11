use std::collections::BTreeMap;

use super::*;

/// Synthetic item data: domain 7 has its own scope, every other domain the
/// default. The numbers and names are made up, as for any game.
struct Data {
    rules: ItemRules,
    bases: BTreeMap<String, Vec<BaseRow>>,
}

impl ItemData for Data {
    fn item_rules(&self) -> &ItemRules {
        &self.rules
    }

    fn base_names(&self) -> impl Iterator<Item = &str> {
        self.bases.keys().map(String::as_str)
    }

    fn base(&self, name: &str) -> Option<(&str, &[BaseRow])> {
        self.bases
            .get_key_value(name)
            .map(|(k, v)| (k.as_str(), v.as_slice()))
    }
}

fn data(rows: &[(&str, i64)]) -> Data {
    let mut bases: BTreeMap<String, Vec<BaseRow>> = BTreeMap::new();
    for (name, domain) in rows {
        bases.entry((*name).to_owned()).or_default().push(BaseRow {
            domain: *domain,
            ..BaseRow::default()
        });
    }
    Data {
        rules: ItemRules {
            default_scope: "main".to_owned(),
            domain_scopes: vec![(7, "special".to_owned()), (9, "special".to_owned())],
            mods: crate::import::mods::ModRules {
                fits: Vec::new(),
                spawn_exclusion: crate::import::mods::Unflagged {
                    section: loom_domain::ModSection::Implicit,
                    flags: Vec::new(),
                },
                spawn_excluded_generations: Vec::new(),
                base_implicit: crate::import::mods::Unflagged {
                    section: loom_domain::ModSection::Implicit,
                    flags: Vec::new(),
                },
                no_spawn_flags: Vec::new(),
                local_carriers: crate::import::mods::LocalCarriers::default(),
                influences: Vec::new(),
            },
            text: TextRules::default(),
        },
        bases,
    }
}

fn query(name: &str) -> BaseQuery<'_> {
    BaseQuery {
        name,
        fallback: None,
    }
}

#[test]
fn find_base_should_use_the_default_scope_for_an_unlisted_domain() {
    let d = data(&[("Plain Ring", 1)]);
    assert_eq!(
        find_base(&d, query("Plain Ring")).map(|f| f.scope),
        Ok("main")
    );
}

#[test]
fn find_base_should_use_the_listed_scope_of_the_domain() {
    let d = data(&[("Odd Charm", 7)]);
    assert_eq!(
        find_base(&d, query("Odd Charm")).map(|f| f.scope),
        Ok("special")
    );
}

#[test]
fn find_base_should_refuse_a_name_that_is_not_a_base() {
    let d = data(&[("Plain Ring", 1)]);
    assert_eq!(find_base(&d, query("Nope")), Err(BaseError::NotABase));
}

#[test]
fn find_base_should_try_the_fallback_name() {
    let d = data(&[("Plain Ring", 1)]);
    let found = find_base(
        &d,
        BaseQuery {
            name: "Plain Ring (Variant)",
            fallback: Some("Plain Ring"),
        },
    );
    assert_eq!(
        found.map(|f| (f.name, f.fallback_used)),
        Ok(("Plain Ring", true))
    );
}

#[test]
fn find_base_should_prefer_the_name_over_the_fallback() {
    let d = data(&[("Plain Ring", 1), ("Other", 7)]);
    let found = find_base(
        &d,
        BaseQuery {
            name: "Other",
            fallback: Some("Plain Ring"),
        },
    );
    assert_eq!(
        found.map(|f| (f.scope, f.fallback_used)),
        Ok(("special", false))
    );
}

#[test]
fn find_base_should_refuse_rows_with_two_scopes() {
    let d = data(&[("Twin", 1), ("Twin", 7)]);
    assert_eq!(find_base(&d, query("Twin")), Err(BaseError::ScopeConflict));
}

#[test]
fn find_base_should_accept_rows_whose_domains_share_a_scope() {
    let d = data(&[("Twin", 7), ("Twin", 9)]);
    let found = find_base(&d, query("Twin"));
    assert_eq!(found.map(|f| (f.scope, f.rows.len())), Ok(("special", 2)));
}

#[test]
fn item_rules_scopes_should_list_the_default_then_each_scope_once() {
    assert_eq!(data(&[]).rules.scopes(), ["main", "special"]);
}

#[test]
fn base_error_codes_should_be_stable() {
    assert_eq!(
        [BaseError::NotABase.code(), BaseError::ScopeConflict.code()],
        ["IMPORT_BASE_NOT_FOUND", "IMPORT_BASE_SCOPE_CONFLICT"]
    );
}

#[test]
fn find_base_should_read_a_name_without_rows_as_no_base() {
    let mut d = data(&[]);
    d.bases.insert("Empty".to_owned(), Vec::new());
    assert_eq!(find_base(&d, query("Empty")), Err(BaseError::NotABase));
}
