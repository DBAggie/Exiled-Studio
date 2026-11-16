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
import struct
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent


def dump(value):
    if isinstance(value, bytes):
        return value
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


def write(name, game, files, data_schema=2):
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
        "data_schema": data_schema,
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


def png(width, height, pixel):
    """A plain RGBA PNG, `pixel(x, y)` giving each pixel's four bytes: the
    synthetic tree's made-up sprite sheet (our own, never GGG's art)."""
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
    rows = b"".join(b"\x00" + b"".join(bytes(pixel(x, y)) for x in range(width)) for y in range(height))
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def tree_sprites(passives):
    """Passive tree plan T-a: a made-up sprite sheet (four 16-pixel squares:
    an icon, a frame, a mastery icon, a jewel frame) and its index, so CI
    covers the bundle's sprites, the app's sprites query, and the canvas."""
    colours = [(200, 160, 60, 255), (90, 90, 90, 255), (60, 120, 200, 255), (160, 60, 160, 255)]

    def pixel(x, y):
        inside = 2 <= x % 16 < 14 and 2 <= y < 14
        return colours[x // 16] if inside else (0, 0, 0, 0)

    sheet = png(64, 16, pixel)

    def at(k, name):
        return {"name": name, "x": 16 * k, "y": 0, "w": 16, "h": 16}

    icons = [at(0, "Test/Icon")]
    frames = sorted(
        [at(1, n) for n in ("KeystoneFrameAllocated", "KeystoneFrameUnallocated", "NotableFrameAllocated",
                            "NotableFrameUnallocated", "PSSkillFrame", "PSSkillFrameActive")]
        + [at(3, "JewelFrameAllocated"), at(3, "JewelFrameUnallocated")],
        key=lambda r: r["name"],
    )
    masteries = [at(2, "Test/Mastery")]
    categories = [{"name": "frame", "sprites": frames}]
    for kind in ("keystone", "normal", "notable"):
        for state in ("Active", "Inactive"):
            categories.append({"name": kind + state, "sprites": icons})
    for name in ("masteryActiveSelected", "masteryConnected", "masteryInactive"):
        categories.append({"name": name, "sprites": masteries})
    for c in categories:
        c.update({"file": "test-sheet.png", "width": 64, "height": 16})
    categories.sort(key=lambda c: c["name"])
    nodes = []
    for row in sorted(passives, key=lambda r: r["node"]):
        if row.get("characters") or row.get("ascendancy_start"):
            continue
        if row.get("mastery_group"):
            nodes.append({"node": row["node"], "inactive_icon": "Test/Mastery", "active_icon": "Test/Mastery"})
        else:
            nodes.append({"node": row["node"], "icon": "Test/Icon"})
    index = {
        "zoom": 0.5,
        "categories": categories,
        "nodes": nodes,
        "source": {
            "repository": "synthetic",
            "tag": "synthetic",
            "commit": "0" * 40,
            "files": [],
            "angle_rule": "synthetic: equal steps",
        },
    }
    return index, sheet


def tree_bundle(files):
    """Passive tree plan T1a: the PoE1 bundle with a larger made-up tree for
    the tree editor's tests and the e2e workflow. Two classes (Test Brute
    with two ascendancies, Test Sage with one), three rows of nine groups
    between the class starts (a ring of six nodes each, notables, two
    masteries, a keystone, jewel sockets), a large cluster socket with a
    nested medium and small socket and their proxies, a multiple choice in
    each ascendancy, the layout constants, the jewel radii, and a made-up
    sprite sheet with its index (`tree_sprites`). Every value
    is made up; only the shapes follow the game's."""
    out = dict(files)
    passives = []
    groups = []

    def stat(v):
        return [{"stat": "test_passive_life", "value": v}]

    def add(node, name, **extra):
        row = {"id": f"t{node}", "node": node, "name": name}
        row.update(extra)
        passives.append(row)

    def tnode(n, orbit, index, links=()):
        return {
            "node": n,
            "orbit": orbit,
            "orbit_index": index,
            "connections": [{"node": t, "orbit": None} for t in links],
        }

    # Class starts.
    add(1000, "Brute Start", characters=["Test/Chars/Str"])
    add(2000, "Sage Start", characters=["Test/Chars/Int"])
    groups.append({"x": -1800.0, "y": 0.0, "flags": 0, "nodes": [tnode(1000, 0, 0)]})
    groups.append({"x": 1800.0, "y": 0.0, "flags": 0, "nodes": [tnode(2000, 0, 0)]})
    # Three rows of nine groups; group (r, c) has the ring
    # 10000 + 1000 r + 10 c + k.
    ring = {}
    for r, y in enumerate((-400.0, 0.0, 400.0)):
        for c in range(9):
            x = -1200.0 + 300.0 * c
            ids = [10000 + 1000 * r + 10 * c + k for k in range(6)]
            ring[(r, c)] = ids
            nodes = []
            for k, n in enumerate(ids):
                if r == 1 and c == 4 and k == 0:
                    add(n, "Test Keystone", keystone=True)
                elif k == 3 and c % 2 == 0:
                    add(n, f"Test Notable {r}-{c}", notable=True, stats=stat(25))
                else:
                    add(n, f"Test Life {r}-{c}-{k}", stats=stat(5))
                nodes.append(tnode(n, 1, k, [ids[(k + 1) % 6]]))
            # Two masteries, each linked from its group's notable.
            if (r, c) in ((0, 2), (2, 6)):
                m = 10000 + 1000 * r + 10 * c + 9
                add(m, "Test Life Mastery", mastery_group="TestLifeMastery")
                nodes.append(tnode(m, 0, 0, [ids[3]]))
            # Two jewel sockets in the middle of their groups.
            if (r, c) in ((1, 2), (1, 6)):
                j = 10000 + 1000 * r + 10 * c + 8
                add(j, "Test Jewel Socket", jewel_socket=True)
                nodes.append(tnode(j, 0, 0, [ids[0]]))
            groups.append({"x": x, "y": y, "flags": 0, "nodes": nodes})
    links = []
    for r in range(3):
        for c in range(8):
            links.append((ring[(r, c)][1], ring[(r, c + 1)][4]))
        if r < 2:
            for c in (0, 4, 8):
                links.append((ring[(r, c)][2], ring[(r + 1, c)][5]))
    links.append((1000, ring[(1, 0)][4]))
    links.append((2000, ring[(1, 8)][1]))
    # Ascendancies far away: a start, a chain, a multiple choice.
    asc = [
        ("TestChampion", "Test Champion", "Test/Chars/Str", 1, -3000.0, -2000.0, 3000),
        ("TestBerserker", "Test Berserker", "Test/Chars/Str", 2, -3000.0, 2000.0, 3100),
        ("TestOracle", "Test Oracle", "Test/Chars/Int", 1, 3000.0, -2000.0, 3200),
    ]
    for aid, name, _, _, x, y, base in asc:
        add(base, name, ascendancy=aid, ascendancy_start=True)
        add(base + 1, f"{name} Small", ascendancy=aid, stats=stat(8))
        add(base + 2, f"{name} Notable", ascendancy=aid, notable=True, stats=stat(40))
        add(base + 3, f"{name} Choice", ascendancy=aid, notable=True, multiple_choice=True)
        add(base + 4, f"{name} Option A", ascendancy=aid, multiple_choice_option=True, stats=stat(12))
        add(base + 5, f"{name} Option B", ascendancy=aid, multiple_choice_option=True, stats=stat(14))
        groups.append(
            {
                "x": x,
                "y": y,
                "flags": 0,
                "nodes": [
                    tnode(base, 0, 0, [base + 1]),
                    tnode(base + 1, 1, 0, [base + 2]),
                    tnode(base + 2, 1, 1, [base + 3]),
                    tnode(base + 3, 1, 2, [base + 4, base + 5]),
                    tnode(base + 4, 1, 3),
                    tnode(base + 5, 1, 4),
                ],
            }
        )
    # A large cluster socket on the main tree (linked to row 2; a plain
    # jewel socket in the passives table, as the game's) and its area, as
    # the game nests them: the large socket's proxy (5005) shares its group
    # with three medium sockets (cluster indices 0 to 2); the first one's
    # proxy (5002) shares its group with a small socket (5003), whose proxy
    # (5004) has a group of its own, as do the other mediums' (5009, 5010).
    add(5000, "Test Large Jewel Socket", jewel_socket=True)
    add(5001, "Test Medium Cluster Socket", jewel_socket=True, expansion=True)
    add(5002, "Test Medium Proxy")
    add(5003, "Test Small Cluster Socket", jewel_socket=True, expansion=True)
    add(5004, "Test Small Proxy")
    add(5005, "Test Large Proxy")
    add(5007, "Test Medium Cluster Socket 2", jewel_socket=True, expansion=True)
    add(5008, "Test Medium Cluster Socket 3", jewel_socket=True, expansion=True)
    add(5009, "Test Medium Proxy 2")
    add(5010, "Test Medium Proxy 3")
    groups.append({"x": 0.0, "y": 900.0, "flags": 0, "nodes": [tnode(5000, 0, 0, [ring[(2, 4)][2], 5005])]})
    groups.append(
        {
            "x": 0.0,
            "y": 1300.0,
            "flags": 0,
            "nodes": [
                tnode(5005, 0, 0, [5001, 5007, 5008]),
                tnode(5001, 1, 0, [5002, 5007]),
                tnode(5007, 1, 2, [5009, 5008]),
                tnode(5008, 1, 4, [5010]),
            ],
        }
    )
    groups.append(
        {"x": 300.0, "y": 1300.0, "flags": 0, "nodes": [tnode(5002, 0, 0, [5003]), tnode(5003, 1, 3, [5004])]}
    )
    groups.append({"x": 300.0, "y": 1700.0, "flags": 0, "nodes": [tnode(5004, 0, 0)]})
    groups.append({"x": -300.0, "y": 1300.0, "flags": 0, "nodes": [tnode(5009, 0, 0)]})
    groups.append({"x": 0.0, "y": 1700.0, "flags": 0, "nodes": [tnode(5010, 0, 0)]})
    # Links stored once, on the first node.
    by_node = {n["node"]: n for g in groups for n in g["nodes"]}
    for a, b in links:
        by_node[a]["connections"].append({"node": b, "orbit": None})
    out["data/ascendancies.json"] = [
        {"id": aid, "name": name, "character": ch, "class_number": number}
        for aid, name, ch, number, *_rest in asc
    ]
    out["data/passives.json"] = sorted(passives, key=lambda row: row["node"])
    out["data/mastery_groups.json"] = [
        {"id": "TestLifeMastery", "effects": ["TestLifeMastery1", "TestLifeMastery2"]}
    ]
    out["data/mastery_effects.json"] = [
        {"id": "TestLifeMastery1", "hash": 4242, "stats": [{"stat": "test_mastery_stat", "value": 5}]},
        {"id": "TestLifeMastery2", "hash": 4243, "stats": [{"stat": "test_passive_life", "value": 20}]},
    ]
    out["data/passive_tree.json"] = {
        "tree": "Default",
        "graph": "Metadata/PassiveSkillGraph",
        # Orbits 2 and 3 hold only the cluster jewels' generated rings
        # (medium on orbit 2, large on orbit 3, as the game's sizes 1 and 2).
        "orbit_slots": [1, 6, 16, 16],
        "roots": [1000, 2000],
        "groups": groups,
    }
    out["data/passive_tree_layout.json"] = {
        "orbit_radii": [0.0, 82.0, 162.0, 335.0],
        "orbit_angles": [
            [0.0],
            [0.0, 60.0, 120.0, 180.0, 240.0, 300.0],
            [22.5 * k for k in range(16)],
            [22.5 * k for k in range(16)],
        ],
        "source": {
            "repository": "synthetic",
            "tag": "synthetic",
            "commit": "0" * 40,
            "files": [],
            "angle_rule": "synthetic: equal steps",
        },
    }
    index, sheet = tree_sprites(passives)
    out["data/passive_tree_sprites.json"] = index
    out["trees/assets/test-sheet.png"] = sheet
    # Copies: the plain bundle shares `files`' lists.
    out["data/stats.json"] = list(files["data/stats.json"])
    out["data/base_items.json"] = list(files["data/base_items.json"])
    out["data/item_classes.json"] = sorted(set(files["data/item_classes.json"]) | {"TestJewel"})
    out["poe1/cluster_jewels.json"] = cluster_jewels(passives, out["data/stats.json"], out["data/base_items.json"])
    out["data/passives.json"] = sorted(passives, key=lambda row: row["node"])
    return out


# The cluster jewel stats the synthetic tree's jewels name (the game's IDs
# for the counts and kinds; made-up ones for the notables and keystone).
CLUSTER_STATS = [
    "local_jewel_expansion_passive_node_count",
    "local_jewel_expansion_jewels_count",
    "local_jewel_expansion_jewels_count_override",
    "local_jewel_expansion_passive_node_index",
    "local_unique_jewel_grants_x_empty_passives",
    "local_affliction_jewel_display_small_nodes_grant_nothing",
    "local_affliction_jewel_small_nodes_have_effect_+%",
    "local_affliction_notable_test_a",
    "local_affliction_notable_test_b",
    "local_jewel_expansion_keystone_test",
]


def cluster_jewels(passives, stats, bases):
    """Passive tree plan T-i: made-up cluster jewel tables in the game's
    shape for the synthetic tree's cluster area (`tree_bundle`). The jewel
    layouts follow the game's index scheme (12 slots for large and medium,
    6 for small, filled in the game's order); the passives, stats, and
    bases they name are added here: one small passive per size, a mastery,
    two notables, a keystone, three jewel bases. Every value is made up."""

    def add(node, name, **extra):
        passives.append({"id": f"t{node}", "node": node, "name": name, "expansion": True, **extra})

    add(6000, "Test Small Fire", stats=[{"stat": "test_passive_life", "value": 3}])
    add(6001, "Test Small Cold", stats=[{"stat": "test_passive_life", "value": 4}])
    add(6002, "Test Small Lightning", stats=[{"stat": "test_passive_life", "value": 5}])
    add(6003, "Test Cluster Mastery")
    add(6010, "Test Cluster Notable A", notable=True, stats=[{"stat": "test_passive_life", "value": 30}])
    add(6011, "Test Cluster Notable B", notable=True, stats=[{"stat": "test_passive_life", "value": 31}])
    add(6020, "Test Cluster Keystone", keystone=True, stats=[{"stat": "test_mastery_stat", "value": 1}])
    stats.extend({"id": s, "local": True} for s in CLUSTER_STATS)
    bases.extend(
        {
            "id": f"Test/Items/Cluster{size}",
            "name": f"{size} Test Cluster Jewel",
            "domain": 1,
            "item_class": "TestJewel",
            "tags": ["jewel"],
            "implicits": [],
        }
        for size in ("Small", "Medium", "Large")
    )
    return {
        "sizes": ["Small", "Medium", "Large"],
        "jewels": [
            {
                "base": "Test/Items/ClusterSmall",
                "size": 0,
                "min_nodes": 2,
                "max_nodes": 3,
                "small_indices": [0, 4, 2],
                "notable_indices": [4],
                "socket_indices": [4],
                "total_indices": 6,
            },
            {
                "base": "Test/Items/ClusterMedium",
                "size": 1,
                "min_nodes": 4,
                "max_nodes": 6,
                "small_indices": [0, 6, 8, 4, 10, 2],
                "notable_indices": [6, 10, 2, 0],
                "socket_indices": [6],
                "total_indices": 12,
            },
            {
                "base": "Test/Items/ClusterLarge",
                "size": 2,
                "min_nodes": 8,
                "max_nodes": 12,
                "small_indices": [0, 4, 6, 8, 10, 2, 7, 5, 9, 3, 11, 1],
                "notable_indices": [6, 4, 8, 10, 2],
                "socket_indices": [4, 8, 6],
                "total_indices": 12,
            },
        ],
        "skills": [
            {"node": 6000, "tag": "test_cluster_fire", "size": 0},
            {"node": 6001, "mastery": 6003, "tag": "test_cluster_cold", "size": 1},
            {"node": 6002, "mastery": 6003, "tag": "test_cluster_lightning", "size": 2},
        ],
        "specials": [
            {"node": 6011, "stat": "local_affliction_notable_test_b", "order": 20},
            {"node": 6010, "stat": "local_affliction_notable_test_a", "order": 10},
            {"node": 6020, "stat": "local_jewel_expansion_keystone_test", "order": 30},
        ],
        "slots": [
            {"socket": 5000, "size": 2, "index": 0, "proxy": 5005, "start_indices": [3, 5, 5]},
            {"socket": 5001, "size": 1, "index": 0, "parent": 5000, "proxy": 5002, "start_indices": [2, 3]},
            {"socket": 5007, "size": 1, "index": 1, "parent": 5000, "proxy": 5009, "start_indices": [1, 1]},
            {"socket": 5008, "size": 1, "index": 2, "parent": 5000, "proxy": 5010, "start_indices": [0, 11]},
            {"socket": 5003, "size": 0, "index": 0, "parent": 5001, "proxy": 5004, "start_indices": [1]},
        ],
    }


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
# B-2 (data schema 4): affix names and levels, a base's drop level and
# attribute requirements, tag display names.
for m in poe1["data/mods.json"]:
    m.update(
        {
            "TestArmour1": {"name": "of Test Plating", "level": 5, "implicit_tags": ["defences"]},
            "TestAccuracy1": {"name": "Steady", "level": 12, "implicit_tags": ["attack"]},
        }.get(m["id"], {})
    )
for b in poe1["data/base_items.json"]:
    if b["id"] == "Test/Items/Ring1":
        b.update({"drop_level": 5, "requirements": {"strength": 0, "dexterity": 10, "intelligence": 0}})
poe1["data/tags.json"] = [
    {"id": "attack", "display": "Attack"},
    {"id": "defences", "display": "Defences"},
]
# Passive tree plan T-f: two made-up jewel radii with their rings, in the
# game table's units (loom_tree::poe1::radius::scaled makes them 960, 960 to
# 1320, 1440, and 1320 to 1680 tree units).
poe1["poe1/jewel_radii.json"] = [
    {"id": "TestSmall", "name": "Small", "radius": 800, "ring_inner": 800, "ring_outer": 1100},
    {"id": "TestMedium", "name": "Medium", "radius": 1200, "ring_inner": 1100, "ring_outer": 1400},
]
# B-2 (Q-B2-1): made-up unique flavour text, one unique with one text (a
# default-colour note after a gap), one whose two variants have their own,
# and one whose two variants print the same text.
poe1["poe1/unique_flavour.json"] = [
    {"name": "Test Flavoured", "id": "TestUnique1", "text": "A made-up line.\nAnother one.\n\n<default>{A made-up note}"},
    {"name": "Test Variants", "id": "TestUnique2", "text": "First variant."},
    {"name": "Test Variants", "id": "TestUnique3", "text": "Second variant."},
    {"name": "Test Same", "id": "TestUnique4", "text": "One text."},
    {"name": "Test Same", "id": "TestUnique5", "text": "One text."},
]
# B-2 item evidence (E9): made-up socket caps in the game's form, a
# body-armour-like chain (3 below item level 25, 6 from 50) for the shield
# and none for the ring.
poe1["poe1/socket_caps.json"] = [
    {"base": "Test/Items/Shield1", "levels": [[1, 1], [2, 1], [3, 2], [4, 25], [5, 35], [6, 50]]},
    {"base": "Test/Items/Ring1", "levels": [[0, 1]]},
]
# B-2 item evidence (E10): made-up tree flavour, the keystones' texts (102
# in this bundle's passives, 11040 in tree_bundle's) and one ascendancy's
# quote.
poe1["poe1/tree_flavour.json"] = {
    "passives": [
        {"node": 102, "text": "A made-up keystone line.\nA second one."},
        {"node": 11040, "text": "A made-up keystone line.\nA second one."},
    ],
    "ascendancies": [{"id": "TestChampion", "text": "A made-up quote.", "colour": "200,180,120"}],
}
write("bundle-poe1", "poe1", poe1, data_schema=4)
write("bundle-poe1-tree", "poe1", tree_bundle(poe1), data_schema=4)
write("bundle-poe2", "poe2", with_skills("poe2", common("poe2", 7, "charm_stat_descriptions")))
print("ok")
