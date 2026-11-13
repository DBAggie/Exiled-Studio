"""Writes the synthetic test bundles (Phase 2 plan, step 3; decision 10),
format 2 with the skill tables (Phase 3 plan, step 1a), the character
and passive tables (step 1b), and the item property and monster tables
(step 1c, monster types in step 1d).

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


SKILL_STATS = [
    "test_spell_flag",
    "test_attack_flag",
    "test_base_damage_min",
    "test_base_damage_max",
    "test_damage_per_level",
    "test_quality_cast_speed",
    "test_support_more_damage",
    "test_flag_stat",
]


def skills(game):
    """The skill tables: a spell, an attack, and a support, in the shape
    each game's converter writes (PoE1: damage effectiveness per level,
    mana reservation; PoE2: damage incremental effectiveness, spirit)."""
    poe1 = game == "poe1"
    reservation = (
        {"resource": "mana", "percent": True, "amount": 2500}
        if poe1
        else {"resource": "spirit", "percent": False, "amount": 30}
    )

    def set_level(level, damage):
        row = {
            "level": level,
            "spell_crit_chance": 600,
            "attack_crit_chance": 0,
            "base_multiplier": 0,
            "float_stats": [
                {
                    "stat": "test_base_damage_min",
                    "value": damage,
                    "resolved": int(damage),
                    "interpolation": 3,
                    "interpolation_base": 1.0,
                },
                {
                    "stat": "test_base_damage_max",
                    "value": damage * 2,
                    "resolved": int(damage * 2),
                    "interpolation": 3,
                    "interpolation_base": 1.0,
                },
            ],
            "stats": [{"stat": "test_damage_per_level", "value": level, "interpolation": 1}],
            "flags": ["test_flag_stat"],
            "granted_effects": [],
        }
        if poe1:
            row["damage_effectiveness"] = 0
        return row

    spell_set = {
        "id": "TestBolt",
        "implicit_stats": ["test_spell_flag"],
        "constant_stats": [{"stat": "test_damage_per_level", "value": 5}],
        "base_effectiveness": 1.5,
        "incremental_effectiveness": 0.03,
        "levels": [set_level(1, 4.5), set_level(2, 6.25)],
    }
    if not poe1:
        spell_set["damage_incremental_effectiveness"] = 0.01
    return {
        "data/skill_types.json": [
            {"id": "TestSpell", "flag_stat": "test_spell_flag"},
            {"id": "TestAttack", "flag_stat": "test_attack_flag"},
            {"id": "TestTotem"},
        ],
        "data/cost_types.json": ["Mana", "Life"],
        "data/active_skills.json": [
            {
                "id": "test_bolt",
                "name": "Test Bolt",
                "types": ["TestSpell"],
                "minion_types": [],
                "weapon_classes": [],
                "manually_cast": True,
            },
            {
                "id": "test_strike",
                "name": "Test Strike",
                "types": ["TestAttack"],
                "minion_types": [],
                "weapon_classes": ["TestRing"] if poe1 else [],
                "manually_cast": True,
            },
        ],
        "data/granted_effects.json": [
            {
                "id": "TestBolt",
                "support": False,
                "active_skill": "test_bolt",
                "cast_time": 750,
                "stat_sets": ["TestBolt"],
                "levels": [
                    {"level": 1, "costs": [{"resource": "Mana", "amount": 6}], "cost_multiplier": 100},
                    {"level": 2, "costs": [{"resource": "Mana", "amount": 7}], "cost_multiplier": 100},
                ],
                "quality": [{"stat": "test_quality_cast_speed", "per_mille": 500}],
            },
            {
                "id": "TestStrike",
                "support": False,
                "active_skill": "test_strike",
                "cast_time": 1000,
                "stat_sets": ["TestStrike"],
                "levels": [{"level": 1, "cost_multiplier": 100, "attack_speed_multiplier": -10}],
            },
            {
                "id": "SupportTestDamage",
                "support": True,
                "allowed_types": ["TestSpell", "TestAttack"],
                "excluded_types": ["TestTotem"],
                "stat_sets": ["SupportTestDamage"],
                "levels": [{"level": 1, "cost_multiplier": 130, "reservations": [reservation]}],
            },
        ],
        "data/stat_sets.json": [
            spell_set,
            {"id": "TestStrike", "base_effectiveness": 1.0, "incremental_effectiveness": 0.0, "levels": []},
            {
                "id": "SupportTestDamage",
                "base_effectiveness": 1.0,
                "incremental_effectiveness": 0.0,
                "levels": [
                    {
                        "level": 1,
                        "stats": [{"stat": "test_support_more_damage", "value": 20}],
                        "granted_effects": [],
                    }
                ],
            },
        ],
        "data/gems.json": [
            {"base_item": "Test/Gems/Bolt", "intelligence": 100, "effects": ["TestBolt"]},
            {"base_item": "Test/Gems/SupportDamage", "strength": 50, "dexterity": 50, "effects": ["SupportTestDamage"]},
        ],
        "data/gem_effects.json": [
            {"id": "TestBolt", "name": "Test Bolt", "granted_effect": "TestBolt", "tags": ["test_spell"]},
            {"id": "SupportTestDamage", "name": "Test Damage Support", "granted_effect": "SupportTestDamage", "tags": ["test_support"]},
        ],
        "data/gem_tags.json": [
            {"id": "test_spell", "name": "Spell"},
            {"id": "test_support", "name": "Support"},
        ],
    }


PASSIVE_STATS = ["test_passive_life", "test_life_per_level", "test_mastery_stat"]


def passives(game):
    """The character and passive tables: two classes, one ascendancy, the
    character template (PoE1 with a class template beside it, PoE2 with a
    non-stat entry, as each game's data has), a small tree."""
    poe1 = game == "poe1"
    templates = [
        {
            "id": "test/chars/character",
            "stats": [{"stat": "test_life_per_level", "value": 12}],
            "values": [] if poe1 else [{"key": "enable_weapon_sets", "value": "true"}],
        }
    ]
    if poe1:
        templates.append({"id": "test/chars/str", "extends": "test/chars/character", "stats": [], "values": []})

    def character(cid, name, number, s, d, i, template):
        row = {
            "id": cid,
            "name": name,
            "class_number": number,
            "base_life": 38,
            "base_mana": 34,
            "strength": s,
            "dexterity": d,
            "intelligence": i,
        }
        if poe1 and template:
            row["template"] = template
        return row

    def node(n, orbit, index, *links):
        return {
            "node": n,
            "orbit": orbit,
            "orbit_index": index,
            "connections": [{"node": t, "orbit": o} for t, o in links],
        }

    return {
        "data/characters.json": [
            character("Test/Chars/Str", "Test Brute", 1, 32, 14, 14, "test/chars/str"),
            character("Test/Chars/Int", "Test Sage", 3, 14, 14, 32, None),
        ],
        "data/ascendancies.json": [
            {"id": "TestChampion", "name": "Test Champion", "character": "Test/Chars/Str", "class_number": 1}
        ],
        "data/object_templates.json": templates,
        "data/passives.json": [
            {"id": "test_start", "node": 100, "name": "Test Start", "characters": ["Test/Chars/Str"]},
            {"id": "test_life", "node": 101, "name": "Test Life", "stats": [{"stat": "test_passive_life", "value": 10}]},
            {"id": "test_keystone", "node": 102, "name": "Test Keystone", "keystone": True},
            {"id": "test_mastery", "node": 103, "name": "Test Mastery", "mastery_group": "TestLifeMastery"},
            {
                "id": "test_champion_start",
                "node": 200,
                "name": "Test Champion",
                "ascendancy": "TestChampion",
                "ascendancy_start": True,
            },
        ],
        "data/mastery_groups.json": [{"id": "TestLifeMastery", "effects": ["TestLifeMastery1"]}],
        "data/mastery_effects.json": [
            {"id": "TestLifeMastery1", "hash": 4242, "stats": [{"stat": "test_mastery_stat", "value": 5}]}
        ],
        "data/passive_tree.json": {
            "tree": "Default",
            "graph": "Metadata/PassiveSkillGraph",
            "orbit_slots": [1, 6],
            "roots": [100],
            "groups": [
                {
                    "x": -1.5,
                    "y": 2.25,
                    "flags": 0,
                    "nodes": [
                        node(100, 0, 0, (101, None)),
                        node(101, 1, 2, (102, -2), (103, None)),
                        node(102, 1, 3),
                        node(103, 1, 4),
                    ],
                },
                {"x": 0.0, "y": 0.0, "flags": 1, "nodes": [node(200, 0, 0)]},
            ],
        },
    }


def properties(game):
    """The item property and monster tables: a wand, a shield (also an
    armour), two monster levels, one game constant. PoE1 stores defences as
    ranges and has the alternative life and conversion columns; PoE2 one
    value per defence and a reload time."""
    poe1 = game == "poe1"

    def defence(lo, hi):
        return {"min": lo, "max": hi if poe1 else lo}

    weapon = {
        "base_item": "Test/Items/Wand1",
        "crit_chance": 700,
        "attack_time": 714,
        "damage_min": 5,
        "damage_max": 9,
        "range": 120,
    }
    if not poe1:
        weapon["reload_time"] = 0

    def level(n, damage, life):
        row = {
            "level": n,
            "damage": damage,
            "evasion": 67 * n,
            "accuracy": 14 * n,
            "life": life,
            "armour": 16 * n,
            "ailment_threshold": life,
            "evasive_evasion": 76 * n,
            "ally_life": 15 * n,
            "ally_damage": 5.62 * n,
            "alt_life_2": 13 * n,
        }
        if poe1:
            row["alt_life_1"] = 10 * n
            row["phys_conversion_multi"] = 0
        return row

    return {
        "data/weapon_types.json": [weapon],
        "data/armour_types.json": [
            {
                "base_item": "Test/Items/Shield1",
                "armour": defence(9, 12),
                "evasion": defence(0, 0),
                "energy_shield": defence(0, 0),
                "ward": defence(0, 0),
                "movement_speed": -3,
            }
        ],
        "data/shield_types.json": [{"base_item": "Test/Items/Shield1", "block": 24}],
        "data/monster_levels.json": [level(1, 4.99, 22), level(2, 5.5, 28)],
        "data/game_constants.json": [
            {"id": "MonsterDamageReductionImprovement", "value": 670, "divisor": 100}
        ],
        "data/monster_types.json": [
            {"id": "TestBossArmoured", "armour": 50, "evasion": 0},
            {"id": "TestBossEvasive", "armour": 0, "evasion": 100},
        ],
    }


def with_skills(game, files):
    files["data/stats.json"].extend({"id": s} for s in SKILL_STATS)
    files["data/base_items.json"].extend(
        [
            {"id": "Test/Gems/Bolt", "name": "Test Bolt", "domain": 1, "item_class": "TestRing", "tags": ["gem"], "implicits": []},
            {"id": "Test/Gems/SupportDamage", "name": "Test Damage Support", "domain": 1, "item_class": "TestRing", "tags": ["gem"], "implicits": []},
        ]
    )
    files.update(skills(game))
    files["data/stats.json"].extend({"id": s} for s in PASSIVE_STATS)
    files.update(passives(game))
    files["data/base_items.json"].extend(
        [
            {"id": "Test/Items/Wand1", "name": "Plain Test Wand", "domain": 1, "item_class": "TestRing", "tags": ["wand"], "implicits": []},
            {"id": "Test/Items/Shield1", "name": "Plain Test Shield", "domain": 1, "item_class": "TestRing", "tags": ["shield"], "implicits": []},
        ]
    )
    files.update(properties(game))
    return files


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
        "format": 2,
        "data_schema": 2,
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


poe1 = with_skills("poe1", common("poe1", 34, "charm_stat_descriptions"))
poe1["poe1/quality_types.json"] = [
    {"id": "TestQuality", "description": "Quality (Test Modifiers)", "stats": [["test_maximum_life", 1]]}
]
poe1["poe1/grafts.json"] = {
    "bases": [{"base": "Plain Test Charm", "graft": "TestGraft1", "gem": "Test/Gems/Bolt", "skill": "Test Bolt"}],
    "levels": [1, 20],
}
# P3-12g: two made-up gods, a major one with an upgrade soul and a minor one.
poe1["poe1/pantheons.json"] = [
    {
        "id": "TestMajorGod",
        "major": True,
        "souls": [
            {"name": "Soul of Test", "stats": [["test_maximum_life", 10]]},
            {"name": "Test Upgrade", "stats": [["test_passive_life", 5], ["test_strength", 2]]},
        ],
    },
    {
        "id": "TestMinorGod",
        "major": False,
        "souls": [{"name": "Soul of Minor Test", "stats": [["test_maximum_life", 4]]}],
    },
]
write("bundle-poe1", "poe1", poe1)
write("bundle-poe2", "poe2", with_skills("poe2", common("poe2", 7, "charm_stat_descriptions")))
print("ok")
