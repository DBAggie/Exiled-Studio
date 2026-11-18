"""Skill stats PoB reads into no output (Phase 6, P6-6a; plan decision 9).

usage: python -I tools/oracle/sweep/skill_stats.py <pob src> <bundle-dir> <calc-run documents dir>...

PoB v2.67.2 reads a skill gem's stats only through stat maps: a skill's
own `statMap` (`Data/Skills/*.lua`) and the global `Data/SkillStatMap.lua`
(`Modules/CalcActiveSkill.lua`); a stat neither names is dropped. Item and
passive lines it reads by their text (`Modules/ModParser.lua`), not by stat
ID. So a stat is read into no output when:
1. no stat map names it (a key `["<stat>"] =` in `SkillStatMap.lua` or in
   any skill's `statMap`);
2. no other PoB file names it: every `.lua` under `src` but the stat
   descriptions (`Data/StatDescriptions`, text only) and the skills'
   constant and per-level stat lists (`{ "<stat>", n }`, `stats = { ... }`,
   which only the stat maps read);
3. no item mod, mastery effect, passive, or base item of the bundle
   carries it (so it reaches the engine only from a skill gem, whose text
   PoB does not parse). A stat that does is reported apart, for a check by
   hand of PoB's reading of its text (`Data/ModCache.lua`).

Prints, for each unmapped stat the calc runs name with a gem source, the
stats that meet all three, sorted, one per line (the `NoEffect` candidates),
and on stderr the counts.
"""
import glob
import json
import os
import re
import sys

pob, bundle = sys.argv[1], sys.argv[2]
docs = sys.argv[3:]

# 1. Stat map keys: `["stat"] = {` anywhere in the skill stat map and the
#    skill files (each skill's `statMap`).
keyed = set()
for f in [os.path.join(pob, 'Data', 'SkillStatMap.lua')] + glob.glob(os.path.join(pob, 'Data', 'Skills', '*.lua')):
    for m in re.finditer(r'\["([^"]+)"\]\s*=\s*\{', open(f, encoding='utf-8').read()):
        keyed.add(m.group(1))
# 2. Every other PoB file, without the stat descriptions and the skills'
#    constant stat lists.
code = ''
for f in glob.glob(os.path.join(pob, '**', '*.lua'), recursive=True):
    norm = f.replace(os.sep, '/')
    if '/Data/StatDescriptions/' in norm or '/Export/' in norm:
        continue
    text = open(f, encoding='utf-8').read()
    if '/Data/Skills/' in norm:
        text = re.sub(r'\{\s*"[^"]+",\s*-?[0-9.]+\s*\}', '', text)
        # The per-level stat name lists (`stats = { "a", "b" }` and kin):
        # names only, which the stat maps read.
        text = re.sub(r'(?:stats|notMinionStat|qualityStats)\s*=\s*\{[^{}]*\}', '', text)
    code += text
# 3. The bundle's item mods and passives.
on_items = set()
mods = json.load(open(os.path.join(bundle, 'data', 'mods.json'), encoding='utf-8'))
for m in mods:
    for s in m.get('stats', []):
        on_items.add(s[0] if isinstance(s, list) else s.get('stat') or s.get('id'))
for table in ('passives', 'mastery_effects', 'base_items'):
    rows = json.load(open(os.path.join(bundle, 'data', table + '.json'), encoding='utf-8'))
    for p in (rows if isinstance(rows, list) else []):
        for key in ('stats', 'implicits'):
            for s in p.get(key, []) or []:
                if isinstance(s, dict):
                    on_items.add(s.get('stat') or s.get('id'))
                elif isinstance(s, list) and s and isinstance(s[0], str):
                    on_items.add(s[0])

seen = set()
for d in docs:
    for f in os.listdir(d):
        r = json.load(open(os.path.join(d, f), encoding='utf-8')).get('result')
        if not r:
            continue
        for x in r['fields']:
            for e in x.get('unsupported', []) or []:
                v = e.get('unmapped-stat')
                if v and 'gem' in v['source']:
                    seen.add(v['stat'])
ok = sorted(s for s in seen if s not in keyed and not re.search(r'(?<![A-Za-z0-9_+%])' + re.escape(s) + r'(?![A-Za-z0-9_+%])', code) and s not in on_items)
sys.stderr.write('%d unmapped gem stats; %d read by no stat map or module and on no item or passive\n' % (len(seen), len(ok)))
for s in ok:
    print(s)
