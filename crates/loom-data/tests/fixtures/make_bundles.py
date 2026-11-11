"""Writes the synthetic test bundles (Phase 2 plan, step 3; decision 10).

Every row is made up for the tests: no game data. Run from the repository
root after changing a row, then commit the output:

    python -I crates/loom-data/tests/fixtures/make_bundles.py

It writes `bundle-poe1/` and `bundle-poe2/` next to this script, with JSON
in sorted-key form and `bundle.json` listing every file's SHA-256 and size.
"""
import hashlib
import json
import shutil
from pathlib import Path

HERE = Path(__file__).resolve().parent


def dump(value):
    return (json.dumps(value, sort_keys=True, indent=1, ensure_ascii=False) + "\n").encode("utf-8")


def stat_file(source, descriptions):
    return {
        "descriptions": descriptions,
        "file_flags": [],
        "includes": [],
        "languages": {},
        "no_description": [],
        "source": source,
    }


def description(line, stats, text, minimum=None, maximum=None):
    """A description of `stats` (a name or a list) with one variant; every
    stat shown as `+d`, the first one's condition `[minimum, maximum]`."""
    stats = [stats] if isinstance(stats, str) else stats
    conditions = [{"kind": "range", "max": None, "min": None} for _ in stats]
    conditions[0] = {"kind": "range", "max": maximum, "min": minimum}
    return {
        "handed": False,
        "line": line,
        "name": None,
        "off_hand_stats": None,
        "stats": stats,
        "variants": [
            {
                "conditions": conditions,
                "flags": [],
                "handlers": [],
                "line": line + 1,
                "placeholders": [{"format": "+d", "index": i} for i in range(len(stats))],
                "text": text,
            }
        ],
    }


# The default scope's descriptions: one line per outcome of the item
# pipeline (`loom-parser/tests/resolve_build.rs`).
DEFAULT_DESCRIPTIONS = [
    description(1, "test_maximum_life", "{0:+d} to maximum Life"),
    # Ambiguous; a mod row backs the first (the filter keeps it).
    description(3, "test_accuracy_a", "{0:+d} to Accuracy"),
    description(5, "test_accuracy_b", "{0:+d} to Accuracy"),
    # Ambiguous; no row backs either.
    description(7, "test_evasion_a", "{0:+d} to Evasion"),
    description(9, "test_evasion_b", "{0:+d} to Evasion"),
    # No variant fits a value outside 1 to 5.
    description(11, "test_strength", "{0:+d} to Strength", minimum=1, maximum=5),
    # The second is shadowed by the first (same stat, earlier).
    description(13, "test_shadow", "{0:+d} to Shadow Test"),
    description(15, "test_shadow", "{0:+d} to Shadowed Text"),
    # A two-line description.
    description(17, ["test_first_half", "test_second_half"], "{0:+d} first half\n{1:+d} second half"),
]


def fit(section, generations, own=True, fixed=(), flag=None, unique=None):
    return {
        "section": section,
        "flag": flag,
        "unique": unique,
        "clauses": [
            {
                "generations": list(generations),
                "domains": {"own": own, "fixed": list(fixed), "related": []},
                "craft_class": False,
                "essence_class": False,
            }
        ],
    }


def item_rules(special_domain, special_scope):
    return {
        "default_scope": "stat_descriptions",
        "domain_scopes": [[special_domain, special_scope]],
        "mods": {
            "fits": [
                fit("explicit", [1, 2]),
                fit("implicit", [3]),
                fit("enchant", [10], fixed=[1]),
            ],
            "spawn_exclusion": {"section": "implicit", "flags": []},
            "spawn_excluded_generations": [],
            "base_implicit": {"section": "implicit", "flags": []},
            "no_spawn_flags": ["crafted"],
            "sections": [],
            "local_carriers": {"weapon": "weapon", "item": "armour"},
            "influences": [],
        },
        "text": {
            "compositions": [["TestPresence", "test_presence"]],
            "property_ids": ["DisplayLastsSeconds"],
            "label_ids": ["TestStrands"],
            "granted_skill_wrapper": "TestGrantedSkill",
        },
    }


def common(game, special_domain, special_scope):
    return {
        "data/stats.json": [
            {"id": "test_maximum_life"},
            {"id": "test_local_armour", "local": True},
        ]
        + [
            {"id": s}
            for s in [
                "test_accuracy_a",
                "test_accuracy_b",
                "test_evasion_a",
                "test_evasion_b",
                "test_strength",
                "test_shadow",
                "test_first_half",
                "test_second_half",
                "test_presence",
            ]
        ],
        "data/mods.json": [
            {
                "id": "TestLife1",
                "domain": 1,
                "generation": 1,
                "stats": [{"stat": "test_maximum_life", "min": 10, "max": 19}],
                "spawn": [{"tag": "default", "weight": 1000}],
            },
            {
                "id": "TestArmour1",
                "domain": 1,
                "generation": 2,
                "stats": [{"stat": "test_local_armour", "min": 5, "max": 9}],
                "spawn": [{"tag": "armour", "weight": 500}],
            },
            {
                "id": "TestAccuracy1",
                "domain": 1,
                "generation": 1,
                "stats": [{"stat": "test_accuracy_a", "min": 10, "max": 20}],
                "spawn": [{"tag": "default", "weight": 1000}],
            },
        ],
        "data/base_items.json": [
            {
                "id": "Test/Items/Ring1",
                "name": "Plain Test Ring",
                "domain": 1,
                "item_class": "TestRing",
                "tags": ["ring", "default"],
                "implicits": ["TestLife1"],
            },
            {
                "id": "Test/Items/Charm1",
                "name": "Plain Test Charm",
                "domain": special_domain,
                "item_class": "TestCharm",
                "tags": ["charm", "default"],
                "implicits": [],
            },
        ],
        "data/item_classes.json": ["TestCharm", "TestRing"],
        "data/client_texts.json": [
            {"id": "DisplayLastsSeconds", "text": "Lasts {0} Seconds"},
            {"id": "TestGrantedSkill", "text": "Grants Level {1} {0} Skill"},
            {"id": "TestPresence", "text": "While a Test Enemy is near, {0}"},
            {"id": "TestStrands", "text": "Test Strands"},
        ],
        "data/lookups.json": [{"handler": "passive_hash", "entries": [[1, "Test Node"]]}],
        "data/item_rules.json": item_rules(special_domain, special_scope),
        "stat_descriptions/stat_descriptions.json": stat_file(
            "Metadata/StatDescriptions/stat_descriptions.txt",
            DEFAULT_DESCRIPTIONS,
        ),
        "stat_descriptions/" + special_scope + ".json": stat_file(
            "Metadata/StatDescriptions/" + special_scope + ".txt",
            [description(1, "test_maximum_life", "{0:+d} to maximum Life")],
        ),
    }


def write(name, game, files):
    out = HERE / name
    if out.exists():
        shutil.rmtree(out)
    entries = []
    for path in sorted(files):
        data = dump(files[path])
        target = out / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        entries.append({"path": path, "sha256": hashlib.sha256(data).hexdigest(), "size": len(data)})
    manifest = {
        "format": 1,
        "data_schema": 1,
        "game": game,
        "source": {
            "steam_build": None,
            "index_sha256": "0" * 64,
            "extractor_commit": "synthetic",
            "decoder": "none",
            "dat_schema": "synthetic",
        },
        "files": entries,
        "signatures": [],
    }
    (out / "bundle.json").write_bytes(dump(manifest))


poe1 = common("poe1", 34, "charm_stat_descriptions")
poe1["data/base_items.json"].append(
    {"id": "Test/Gems/Bolt", "name": "Test Bolt", "domain": 1, "item_class": "TestRing", "tags": ["gem"], "implicits": []}
)
poe1["poe1/quality_types.json"] = [
    {"id": "TestQuality", "description": "Quality (Test Modifiers)", "stats": [["test_maximum_life", 1]]}
]
poe1["poe1/grafts.json"] = {
    "bases": [{"base": "Plain Test Charm", "graft": "TestGraft1", "gem": "Test/Gems/Bolt", "skill": "Test Bolt"}],
    "levels": [1, 20],
}
write("bundle-poe1", "poe1", poe1)
write("bundle-poe2", "poe2", common("poe2", 7, "charm_stat_descriptions"))
print("ok")
