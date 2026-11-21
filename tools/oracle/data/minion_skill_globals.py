# loom (Phase 6, P6-9): maintainer-run, reads PoB's data export (pob_data.lua, format 13); original code of this project (MIT).
"""For each minion-making active skill, whether any of its minions' skills
(PoB's minion templates and its own minion list) has a GlobalEffect mod.
usage: python minion_skill_globals.py <pob-data.json>"""
import json, sys
d = json.load(open(sys.argv[1], encoding='utf-8'))
skills = d['skills']
tmpl = d.get('minion_templates', {})
mins = d.get('minions', {})

def globals_of(sid):
    s = skills.get(sid)
    if not s:
        return None
    out = []
    for st in s.get('mods', []):
        mods = st.get('mods') or []
        if isinstance(mods, dict):
            mods = list(mods.values())
        for m in mods:
            t = m.get('tags')
            items = t if isinstance(t, list) else ([t] if isinstance(t, dict) and t else [])
            for x in items:
                if isinstance(x, dict) and x.get('type') == 'GlobalEffect':
                    out.append((st.get('stat'), m.get('name'), x.get('effectType'), x.get('applyNotPlayer')))
    return out

for sid, s in sorted(skills.items()):
    ml = s.get('minions') or []
    if isinstance(ml, dict):
        ml = list(ml.values())
    if not ml:
        continue
    if s.get('support'):
        continue
    bad = []
    for m in ml:
        minfo = mins.get(m) or {}
        ms = minfo.get('skillList') or minfo.get('skills') or []
        t = tmpl.get(m) or {}
        ms = list(ms) + list(t.get('skills') or [])
        for k in ms:
            g = globals_of(k)
            if g is None:
                bad.append((m, k, 'missing'))
            elif g:
                bad.append((m, k, g))
    print(sid, 'OK' if not bad else 'GLOBAL', ml[:4], bad[:3])
