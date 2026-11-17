"""Skill sweep builds (Phase 6, P6-6a): one minimal PoB build per main
skill the engine computes as a plain hit, to compare every such skill with
the oracle (docs/phase-6/skill-data.md).

usage: python -I tools/oracle/sweep/make_sweep.py <bundle-dir> <pob_skills.rs> <out-dir>

For each skill gem's granted effect (bundle `data/gems.json`,
`gem_effects.json`, `granted_effects.json`, `active_skills.json`):
- a self-cast spell (types Spell with Damage or Projectile, none of
  NOT_SELF_CAST, a cast time, no cooldown at the level): a level 90 Witch
  with the gem at level 20 and nothing else;
- a plain attack (type Attack, none of NOT_PLAIN_ATTACK) that a one-hand
  axe can use (no weapon classes, or "One Hand Axe" among them): a level 90
  Templar with a Rusted Hatchet;
and one build per PoB part when PoB's skill data has parts (the gem's
`skillPart`). Every build is original and made up from the game's IDs: no
PoB or corpus build. The builds are local evidence, not committed.
"""
import json
import os
import re
import sys

bundle, pob_rs, out = sys.argv[1:4]

NOT_SELF_CAST = ["Attack", "Channel", "Triggered", "SummonsTotem", "Trapped",
                 "RemoteMined", "Minion", "CreatesMinion", "Brand"]
NOT_PLAIN_ATTACK = NOT_SELF_CAST[1:]


def load(name):
    return json.load(open(os.path.join(bundle, 'data', name + '.json'), encoding='utf-8'))


gems = load('gems')
effects = {e['id']: e for e in load('gem_effects')}
granted = {g['id']: g for g in load('granted_effects')}
active = {a['id']: a for a in load('active_skills')}
bases = {b['id']: b for b in load('base_items')}

# PoB's part counts and names, from the generated skill table.
src = open(pob_rs, encoding='utf-8').read()
parts = {}
for m in re.finditer(r'id: "([^"]+)",\s*flags: &\[[^\]]*\],\s*parts: &\[(.*?)\],\s*base_mods', src, re.S):
    parts[m.group(1)] = re.findall(r'\("name", V::Str\("((?:[^"\\]|\\.)*)"\)\)', m.group(2))

HEAD = '<?xml version="1.0" encoding="UTF-8"?>\n'


def build(class_name, class_id, gem_xml, items, slots, part, note):
    return (HEAD + '<!-- Skill sweep build (loom, Phase 6 P6-6a): ' + note +
            '. Made up from the game\'s IDs; not from PoB or the corpus. -->\n'
            '<PathOfBuilding>\n'
            '\t<Build level="90" targetVersion="3_0" className="%s" ascendClassName="None" '
            'mainSocketGroup="1" bandit="None" pantheonMajorGod="None" pantheonMinorGod="None"/>\n'
            '\t<Tree activeSpec="1">\n\t\t<Spec treeVersion="3_29" classId="%d" ascendClassId="0" nodes=""/>\n\t</Tree>\n'
            '\t<Skills activeSkillSet="1">\n\t\t<SkillSet id="1">\n'
            '\t\t\t<Skill mainActiveSkill="1" mainActiveSkillCalcs="1" enabled="true" includeInFullDPS="true">\n'
            '%s\n\t\t\t</Skill>\n\t\t</SkillSet>\n\t</Skills>\n'
            '%s\n'
            '\t<Config activeConfigSet="1">\n\t\t<ConfigSet id="1" title="Default"/>\n\t</Config>\n'
            '</PathOfBuilding>\n') % (class_name, class_id, gem_xml, items if items else '\t<Items/>')


HATCHET = ('\t<Items activeItemSet="1">\n\t\t<Item id="1">\nRarity: NORMAL\nRusted Hatchet\nQuality: 0\nImplicits: 0\n\t\t</Item>\n'
           '\t\t<ItemSet id="1">\n\t\t\t<Slot name="Weapon 1" itemId="1"/>\n\t\t</ItemSet>\n\t</Items>')

os.makedirs(out, exist_ok=True)
made = []
for gem in gems:
    base = bases.get(gem['base_item'])
    if base is None:
        continue
    for eid in gem['effects']:
        e = effects.get(eid)
        if e is None:
            continue
        g = granted.get(e['granted_effect'])
        if g is None or g.get('active_skill') not in active:
            continue
        a = active[g['active_skill']]
        types = set(a['types']) | set(g.get('added_types') or [])
        lvl = next((r for r in g['levels'] if r['level'] == 20), None)
        if lvl is None:
            continue
        spell = ('Spell' in types and ('Damage' in types or 'Projectile' in types)
                 and not any(t in types for t in NOT_SELF_CAST)
                 and g.get('cast_time', 0) > 0 and lvl.get('cooldown', 0) == 0)
        attack = ('Attack' in types and not any(t in types for t in NOT_PLAIN_ATTACK)
                  and (not a['weapon_classes'] or 'One Hand Axe' in a['weapon_classes']))
        if not (spell or attack):
            continue
        name = e['name'] or base['name']
        gid = e['granted_effect']
        for p in range(max(1, len(parts.get(gid, [])))):
            part = p + 1 if parts.get(gid) else None
            attr = ' skillPart="%d"' % part if part else ''
            gem_xml = ('\t\t\t\t<Gem nameSpec="%s" skillId="%s" variantId="%s" gemId="%s" level="20" quality="0" '
                       'enabled="true" enableGlobal1="true" count="1"%s/>') % (
                name.replace('"', '&quot;'), gid, eid, gem['base_item'], attr)
            fname = 'sweep-%s%s.xml' % (gid, '-part%d' % part if part else '')
            note = '%s (%s), level 20%s' % (name, gid, ', part %d' % part if part else '')
            if spell:
                text = build('Witch', 3, gem_xml, None, None, part, note)
            else:
                text = build('Templar', 5, gem_xml, HATCHET, None, part, note)
            open(os.path.join(out, fname), 'w', encoding='utf-8', newline='\n').write(text)
            made.append(fname)
print(len(made), 'builds')
