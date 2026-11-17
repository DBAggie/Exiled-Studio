"""Skill stats PoB reads into no output (Phase 6, P6-6a; plan decision 9).

usage: python -I tools/oracle/sweep/skill_stats.py <pob src> <bundle-dir> <calc-run documents dir>...

PoB v2.67.2 reads a skill gem's stats only through stat maps: a skill's
own `statMap` (`Data/Skills/*.lua`) and the global `Data/SkillStatMap.lua`
(`Modules/CalcActiveSkill.lua`); a stat neither names is dropped. Item and
passive lines it reads by their text (`Modules/ModParser.lua`), not by stat
ID. So a stat is read into no output when:
1. no stat map names it (a key `["<stat>"] =` in `SkillStatMap.lua` or in
   any skill's `statMap`);
2. no module names it (`Modules/*.lua`, `Classes/*.lua`);
3. no item mod and no passive of the bundle carries it (so it reaches the
   engine only from a skill gem, whose text PoB does not parse).

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
# 2. Module and class text.
code = ''
for f in glob.glob(os.path.join(pob, 'Modules', '*.lua')) + glob.glob(os.path.join(pob, 'Classes', '*.lua')):
    code += open(f, encoding='utf-8').read()
# 3. The bundle's item mods and passives.
on_items = set()
mods = json.load(open(os.path.join(bundle, 'data', 'mods.json'), encoding='utf-8'))
for m in mods:
    for s in m.get('stats', []):
        on_items.add(s[0] if isinstance(s, list) else s.get('stat') or s.get('id'))
passives = json.load(open(os.path.join(bundle, 'data', 'passives.json'), encoding='utf-8'))
for p in (passives if isinstance(passives, list) else passives.get('passives', [])):
    for s in p.get('stats', []):
        on_items.add(s[0] if isinstance(s, list) else s.get('stat') or s.get('id'))

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
