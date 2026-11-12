"""Compare a PoE1 bundle's resolved base values (the game's
BaseResolvedValues) of effectiveness-scaled float stats (interpolation 3)
with PoB v2.67.2's formula (calcLib.buildSkillInstanceStats) at each level's
levelRequirement, which the bundle does not carry. P3-9b's evidence for
taking the game's values (docs/phase-3/spell-hits.md); maintainer-run.

usage: python -I tools/parity-cli/scripts/check-skill-resolved.py <pob src/Data> <bundle>/data

<pob src/Data> is `git archive v2.67.2 src/Data` of the legacy repository
(read only; local/pob-v2.67.2/src/Data on the workstation). Prints JSON:
rows compared, equal, different, the row closest to a rounding tie, the
first differences, and the skills with a difference.
"""
import glob
import json
import math
import re
import sys

pob, bundle = sys.argv[1], sys.argv[2]
misc = open(pob + "/Misc.lua", encoding="utf-8").read()
base_eff = float(re.search(r'\["SkillDamageBaseEffectiveness"\] = ([0-9.e+-]+)', misc).group(1))
inc_eff = float(re.search(r'\["SkillDamageIncrementalEffectiveness"\] = ([0-9.e+-]+)', misc).group(1))

skills = {}
for path in glob.glob(pob + "/Skills/*.lua"):
    text = open(path, encoding="utf-8").read()
    for m in re.finditer(r'skills\["([^"]+)"\] = \{(.*?)\n\}', text, re.S):
        sid, body = m.group(1), m.group(2)
        be = re.search(r"\n\tbaseEffectiveness = ([0-9.e+-]+)", body)
        ie = re.search(r"\n\tincrementalEffectiveness = ([0-9.e+-]+)", body)
        levels = {}
        for lm in re.finditer(r"\n\t\t\[(\d+)\] = \{(.*?)\},\n", body):
            line = lm.group(2)
            req = re.search(r"levelRequirement = (\d+)", line)
            interp = re.search(r"statInterpolation = \{ ([^}]*)\}", line)
            # leading positional numbers
            pos = []
            for tok in line.split(","):
                tok = tok.strip()
                if re.fullmatch(r"-?[0-9.]+(e[+-]?\d+)?", tok):
                    pos.append(float(tok))
                else:
                    break
            levels[int(lm.group(1))] = (
                int(req.group(1)) if req else None,
                [int(x) for x in interp.group(1).replace(",", " ").split()] if interp else [],
                pos,
            )
        skills[sid] = (
            float(be.group(1)) if be else 1.0,
            float(ie.group(1)) if ie else 0.0,
            levels,
        )

ge = {g["id"]: g for g in json.load(open(bundle + "/granted_effects.json"))}
ss = {s["id"]: s for s in json.load(open(bundle + "/stat_sets.json"))}

def lua_round(x):
    return math.floor(x + 0.5)

compared = equal = 0
diffs = []
margin = (1.0, None)
for sid, (be, ie, levels) in sorted(skills.items()):
    g = ge.get(sid)
    if not g or not g["stat_sets"]:
        continue
    s = ss.get(g["stat_sets"][0])
    if not s:
        continue
    for row in s["levels"]:
        lv = levels.get(row["level"])
        if not lv:
            continue
        req, interp, pos = lv
        for i, f in enumerate(row["float_stats"]):
            if f["interpolation"] != 3 or i >= len(pos) or i >= len(interp) or interp[i] != 3:
                continue
            actor = req or 1
            eff = (base_eff + inc_eff * (actor - 1)) * be * (1 + ie) ** (actor - 1)
            raw = eff * pos[i]
            pob_value = lua_round(raw)
            compared += 1
            frac = abs((raw - math.floor(raw)) - 0.5)
            if frac < margin[0]:
                margin = (frac, (sid, row["level"], f["stat"], raw))
            if pob_value == f["resolved"]:
                equal += 1
            else:
                diffs.append((sid, row["level"], f["stat"], f["resolved"], pob_value, raw))

print(json.dumps({
    "compared": compared,
    "equal": equal,
    "different": len(diffs),
    "closest_to_half": {"distance": margin[0], "at": margin[1]},
    "first_differences": diffs[:40],
    "skills_with_differences": sorted({d[0] for d in diffs}),
}, indent=1))
