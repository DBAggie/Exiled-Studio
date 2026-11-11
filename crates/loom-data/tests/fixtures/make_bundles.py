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


def description(line, stat, text):
    return {
        "handed": False,
        "line": line,
        "name": None,
        "off_hand_stats": None,
        "stats": [stat],
        "variants": [
            {
                "conditions": [{"kind": "range", "max": None, "min": None}],
                "flags": [],
                "handlers": [],
                "line": line + 1,
                "placeholders": [{"format": "+d", "index": 0}],
                "text": text,
            }
        ],
    }


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
            "local_carriers": {"weapon": "weapon", "item": "armour"},
            "influences": [],
        },
        "text": {
            "compositions": [],
            "property_ids": ["DisplayLastsSeconds"],
            "label_ids": [],
            "granted_skill_wrapper": None,
        },
    }


def common(game, special_domain, special_scope):
    return {
        "data/stats.json": [
            {"id": "test_maximum_life"},
            {"id": "test_local_armour", "local": True},
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
        "data/client_texts.json": [{"id": "DisplayLastsSeconds", "text": "Lasts {0} Seconds"}],
        "data/lookups.json": [{"handler": "passive_hash", "entries": [[1, "Test Node"]]}],
        "data/item_rules.json": item_rules(special_domain, special_scope),
        "stat_descriptions/stat_descriptions.json": stat_file(
            "Metadata/StatDescriptions/stat_descriptions.txt",
            [description(1, "test_maximum_life", "{0:+d} to maximum Life")],
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
