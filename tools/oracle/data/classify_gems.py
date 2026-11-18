# loom (Phase 6, P6-9): maintainer-run, reads PoB's data export (pob_data.lua, format 13); original code of this project (MIT).
"""Classify unmodelled gems with PoB's export (format 13): inert candidates
(no GlobalEffect mod, no blocked mod name, no forbidden type).
usage: python classify_gems.py <pob-data.json> <list> [<list> ...]"""
import json, sys, re

d = json.load(open(sys.argv[1], encoding='utf-8'))
skills = d['skills']
gems = d.get('gems', {})
want = {}
for path in sys.argv[2:]:
    for line in open(path, encoding='utf-8'):
        n, sid = line.split(None, 1)
        want[sid.strip()] = want.get(sid.strip(), 0) + int(n)

BLOCKED = set("""AuraBuffEffect AuraEffect AuraEffectOnSelf BloodMagicReserved BuffEffect BuffEffectOnPlayer BuffEffectOnSelf ChillBase ChillMinimum ChillOverride EnemyChillEffect ExtraAura ExtraAuraEffect ExtraSkillMod ExtraSupport HasUncancellableReservation LifeCostGainAsReservation LifeReservationEfficiency LifeReserved ManaCostGainAsReservation ManaReservationEfficiency ManaReserved ReservationEfficiency Reserved SkillAuraEffectOnSelf SupportManaMultiplier""".split())
INERT_FORBIDDEN = set("Aura Brand Buff Herald Hex Mark Triggered Trigger Warcry".split())
SUPPORT_FORBIDDEN = set("AppliesCurse Aura Brand Buff CreatesMinion HasReservation Herald Hex Mark Minion ReservationBecomesCost SummonsTotem Totem Trigger Triggered Warcry".split())
DISPLAY = set("""ColdMax FireDot ColdMin PhysicalMax PhysicalMin allowTotemBuff baseStunDuration buffAllies debuff debuffSecondary dotIsArea duration radius radiusExtra radiusLabel radiusSecondary radiusSecondaryLabel radiusTertiary radiusTertiaryLabel RFManaMultiplier repeatFrequency setOffHandPhysicalMax setOffHandPhysicalMin showAverage thisIsNotABuff buffMinions buffNotPlayer dotIsSpell durationSecondary hoaMoreBurn hoaOverkillPercent ChaosDot ChaosMax ChaosMin ColdDot FireMax FireMin LightningMax LightningMin PhysicalDot corpseExplosionLifeMultiplier damageInterval dotIsProjectile dpsMultiplier explodeCorpse fixedCastTime hitChanceIsExplodeChance hitFrequency""".split())

def tags_global(m):
    t = m.get('tags')
    items = t if isinstance(t, list) else ([t] if isinstance(t, dict) and t else [])
    return [x for x in items if isinstance(x, dict) and x.get('type') == 'GlobalEffect']

out = {'inert': [], 'support-inert': [], 'other': []}
variant = {}
byname = {}
for g in gems.values():
    variant.setdefault(g.get('variantId'), g.get('grantedEffectId'))
    byname.setdefault(g.get('name'), g.get('grantedEffectId'))
for sid0, n in sorted(want.items(), key=lambda x: -x[1]):
    sidp, _, nm = sid0.partition('|')
    sid = sidp if sidp in skills else variant.get(sidp) or byname.get(nm) or byname.get(nm + ' Support') or sidp
    s = skills.get(sid)
    if not s:
        out['other'].append((n, sid, 'not in export'))
        continue
    support = bool(s.get('support'))
    glob_ = []
    blocked = []
    for st in s.get('mods', []):
        mods = st.get('mods') or []
        if isinstance(mods, dict):
            mods = list(mods.values())
        for m in mods:
            if tags_global(m):
                glob_.append(st.get('stat'))
            elif m.get('name') in BLOCKED:
                blocked.append(m.get('name'))
            elif m.get('name') == 'SkillData':
                key = (m.get('value_fields') or {}).get('key')
                if key and key not in DISPLAY:
                    blocked.append('SkillData:' + str(key))
    types = set(s.get('add_types') or []) if support else set(s.get('types') or [])
    if isinstance(s.get('add_types'), dict):
        types = set(s['add_types'].keys()) if support else types
    forb = (SUPPORT_FORBIDDEN if support else INERT_FORBIDDEN) & types
    why = []
    if glob_: why.append('global:' + ','.join(sorted(set(map(str, glob_)))))
    if blocked: why.append('blocked:' + ','.join(sorted(set(blocked))))
    if forb: why.append('types:' + ','.join(sorted(forb)))
    if s.get('is_trigger'): why.append('is_trigger')
    if s.get('minions') or s.get('add_minions'):
        if s.get('minions'): why.append('minions')
        if s.get('add_minions'): why.append('add_minions')
    if why:
        out['other'].append((n, sid, ' '.join(why)))
    else:
        out['support-inert' if support else 'inert'].append((n, sid, ''))
for k, v in out.items():
    print('==', k, len(v))
    for n, sid, why in v:
        print(n, sid, why)
