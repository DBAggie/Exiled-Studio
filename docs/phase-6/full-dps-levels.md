# Open items S4: Blood and Sand, and PoB's full DPS pass

Blood and Sand itself is modelled since P6-11dw (`docs/phase-6/stat-mop-up.md`, "P6-11dw"): melee skills' more area damage in Blood Stance, more area of effect and less area damage in Sand Stance, under PoB's `bloodSandStance` config. What stayed open was a difference found in P6-11eu: with Blood and Sand present, PoB's full DPS pass gives Smite another value than its own main calculation. P6-11eu guessed that `calcFullDPS` reuses one environment, so a mod one pass adds is seen by the next, with the stance buffs the likely carriers. The plan's S4 asked to name that mod with an instrumented PoB run.

## What the instrumented run found

**No mod carries over.** Every modifier, condition, multiplier, and config value is the same in both passes. What differs is **where** PoB keeps the player's modifiers, and PoB rounds per place:

- **Main calculation** (`Calcs.lua` 419, `calcs.initEnv(build, mode)` with no cache): the base and config modifiers go into `env.modDB` itself (`CalcSetup.lua` 488 to 585). The buffs `calcs.perform` adds later (Blood and Sand's stance, `effectCond` `BloodStance` or `SandStance`) go into the same ModDB.
- **Full DPS pass** (`Calcs.lua` 425 and 335, `calcFullDPS` with the cached databases): `env.modDB.parent = cachedPlayerDB` (`CalcSetup.lua` 587). The base modifiers stay in the cached parent; the buffs go into the pass's own, new ModDB.
- **The rounding** (`Classes/ModDB.lua` 157 to 194, `MoreInternal`): for each ModDB level, PoB multiplies the level's MORE modifiers and rounds the product to 2 decimals (`round(modResult, 2)`), then multiplies by the parent level's result.

So two MORE modifiers of one name round together in the main calculation and separately in full DPS. In the fixtures below, Smite's own −20% is in the skill's level in both passes; "4% more Damage per Frenzy Charge" (`Base`, two charges) and the stance's `Damage` MORE are in one level in the main pass and in two in full DPS:

| Stance | Main calculation | Full DPS pass |
|---|---|---|
| Blood (+27%) | 0.80 × round(1.27 × 1.08, 2) = 0.80 × 1.37 = **1.096** | 0.80 × 1.27 × 1.08 = **1.09728** |
| Sand (−5%) | 0.80 × round(0.95 × 1.08, 2) = 0.80 × 1.03 = **0.824** | 0.80 × 0.95 × 1.08 = **0.8208** |

Every output difference follows from that one multiplier: the per-type hit averages, the average hit and damage, impale's stored hit, and `TotalDPS`. It is not specific to Blood and Sand. Any build whose included skill has a buff's MORE modifier and a base MORE modifier of the same name can show it, when the rounding falls differently.

## The decision: a `pob-bug` entry

The game has one damage per second per skill. PoB's full DPS pass gives the same skill, in the same configuration, a value its own main calculation does not, from where its ModDB happens to hold a modifier. The engine counts each included skill's own main calculation (`poe1::full_dps`, P6-12 part 2), so its `FullDPS` equals its `TotalDPS` here. The plan's rule for S4 applies: a PoB carry-over that is a PoB bug is registered as `pob-bug` with the instrumented run as evidence. Nothing is ported.

- **DIV-0712** (`full-dps-blood-stance-smite-templar.xml`): PoB's `FullDPS` 127.763, its own `TotalDPS` 127.668; the engine 127.604 (its `TotalDPS`, with the registered Q11 attack rate).
- **DIV-0718** (`full-dps-sand-stance-smite-templar.xml`): PoB's `FullDPS` 95.589, its own `TotalDPS` 96.008; the engine 95.960.
- **Test:** `crates/loom-calc/tests/hit.rs::full_dps_should_add_the_skills_own_hit_dps` (an included skill's full DPS is its own hit DPS with a buff present).

Whether the game rounds more multipliers at all is not settled by this. The main calculation's rounding is PoB's, ported as it is; that is not on the game-truth list, since the engine and PoB's main calculation agree.

## Fixtures

Two original fixtures (Smite, level 20, Area Hit, a normal Siege Axe, counted in full DPS; Blood and Sand level 20 in its own group; two frenzy charges), one per stance:

- `full-dps-blood-stance-smite-templar.xml` (Blood Stance, PoB's default);
- `full-dps-sand-stance-smite-templar.xml` (`bloodSandStance` `SAND`).

On each, the main skill matches PoB but for the registered Q11 attack rate: `Speed`, `TotalDPS`, `ManaPerSecondCost`, `CombinedDPS`, and the Blood fixture's `TotalDPS` breakdown line (DIV-0709 to DIV-0711, DIV-0713 to DIV-0717, DIV-0719). Each fixture has 73 targets matching PoB. The P6-11dw fixture `smite-blood-and-sand-templar.xml` keeps Smite out of full DPS and is unchanged.

## The instrumented run

**How.** No PoB file was changed: neither `local/oracle/pob` nor `D:\POE Dev\PathOfBuilding`. A scratchpad copy of the oracle adapter (`tools/oracle/adapter/oracle.lua`) got one more stage after `calculate`, spliced in before `local stages = {` and listed after `{ "calculate", calculate },`. The stage wraps `build.calcsTab.calcs.perform`, runs `calcs.buildOutput(build, "MAIN")` once more, and writes each `perform` call's dump next to the output file (`.pass1.txt` the main calculation, `.pass2.txt` the full DPS pass). It was run as `oracle-run` runs the adapter: the pinned LuaJIT, the working directory `local/oracle/pob/src`, a cleared environment with `LUA_PATH="./?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua"`, and `zlib=` PoB's `runtime/zlib1.dll`.

**The stage** (`probe`, the whole addition to the adapter, as run; `openFile`, `outputPath`, `traceback`, and `fail` are the adapter's own locals):

```lua
-- S4 probe (scratchpad only; spliced into a copy of the adapter before
-- `local stages =`). After PoB has loaded the fixture, wraps
-- calcs.perform and runs calcs.buildOutput(build, "MAIN") once more. Each
-- perform call dumps the main skill's skillModList, the player modDB (with
-- its parent chain), and a few outputs to <output.json>.pass<N>.txt. No
-- PoB file is changed.
local function probe()
	local calcs = build.calcsTab.calcs
	local origPerform = calcs.perform
	local pass = 0
	local function fmtTags(mod)
		local parts = {}
		for i = 1, #mod do
			local t = mod[i]
			local kv = {}
			for k, v in pairs(t) do
				kv[#kv + 1] = tostring(k) .. "=" .. tostring(v)
			end
			table.sort(kv)
			parts[#parts + 1] = "{" .. table.concat(kv, ",") .. "}"
		end
		return table.concat(parts, "")
	end
	local function fmtMod(mod)
		local v = mod.value
		if type(v) == "table" then
			local kv = {}
			for k, x in pairs(v) do kv[#kv + 1] = tostring(k) .. "=" .. tostring(x) end
			table.sort(kv)
			v = "{" .. table.concat(kv, ",") .. "}"
		end
		return string.format("%s|%s|%s|f%s|k%s|%s|%s", mod.name, mod.type, tostring(v),
			tostring(mod.flags), tostring(mod.keywordFlags), tostring(mod.source), fmtTags(mod))
	end
	local function dumpDB(db, label, lines, depth)
		if not db or depth > 5 then return end
		local mods = db.mods or {}
		for name, list in pairs(mods) do
			for _, mod in ipairs(list) do
				lines[#lines + 1] = label .. depth .. " " .. fmtMod(mod)
			end
		end
		dumpDB(db.parent, label, lines, depth + 1)
	end
	calcs.perform = function(env, skipEHP)
		origPerform(env, skipEHP)
		pass = pass + 1
		local lines = {}
		local skill = env.player.mainSkill
		lines[#lines + 1] = "# mode=" .. tostring(env.mode) .. " skipEHP=" .. tostring(skipEHP)
			.. " skill=" .. tostring(skill.activeEffect.grantedEffect.name)
		for _, k in ipairs({ "TotalDPS", "Speed", "AverageHit", "AverageDamage", "HitChance", "CritChance",
			"PhysicalMin", "PhysicalMax", "Duration", "AreaOfEffectMod", "ManaCost", "ManaPerSecondCost" }) do
			lines[#lines + 1] = "out " .. k .. " " .. tostring(env.player.output[k])
		end
		local cond = {}
		for k, v in pairs(env.modDB.conditions or {}) do cond[#cond + 1] = tostring(k) .. "=" .. tostring(v) end
		table.sort(cond)
		lines[#lines + 1] = "cond " .. table.concat(cond, ",")
		local body = {}
		for k, v in pairs(env.player.output) do
			if type(v) ~= "table" then body[#body + 1] = "pout " .. tostring(k) .. " " .. tostring(v) end
		end
		for k, v in pairs(env.player.output.MainHand or {}) do
			if type(v) ~= "table" then body[#body + 1] = "mh " .. tostring(k) .. " " .. tostring(v) end
		end
		body[#body + 1] = "part " .. tostring(skill.skillPart) .. " " .. tostring(skill.skillPartName)
		local cfg = skill.weapon1Cfg or skill.skillCfg
		local db = skill.skillModList
		for _, n in ipairs({ "Damage", "PhysicalDamage", "LightningDamage", "ElementalDamage", "PhysicalMin", "PhysicalMax", "LightningMin", "LightningMax", "AreaOfEffect", "Speed", "DamageEffect" }) do
			body[#body + 1] = string.format("eval %s INC=%s MORE=%s BASE=%s", n,
				tostring(db:Sum("INC", cfg, n)), tostring(db:More(cfg, n)), tostring(db:Sum("BASE", cfg, n)))
			for _, t in ipairs({ "INC", "MORE", "BASE" }) do
				for _, row in ipairs(db:Tabulate(t, cfg, n)) do
					body[#body + 1] = string.format("tab %s %s %s %s", n, t, tostring(row.value), tostring(row.mod.source))
				end
			end
		end
		for k, v in pairs(cfg) do
			if type(v) ~= "table" then body[#body + 1] = "cfg " .. tostring(k) .. " " .. tostring(v) end
		end
		for k, v in pairs(env.modDB.multipliers or {}) do body[#body + 1] = "mult " .. tostring(k) .. " " .. tostring(v) end
		for k, v in pairs(env.enemy.output or {}) do
			if type(v) ~= "table" then body[#body + 1] = "eout " .. tostring(k) .. " " .. tostring(v) end
		end
		for k, v in pairs(skill.skillData or {}) do
			if type(v) ~= "table" then body[#body + 1] = "sdata " .. tostring(k) .. " " .. tostring(v) end
		end
		for k, v in pairs(skill.skillFlags or {}) do
			body[#body + 1] = "sflag " .. tostring(k) .. " " .. tostring(v)
		end
		dumpDB(skill.skillModList, "skill", body, 0)
		dumpDB(env.modDB, "player", body, 0)
		dumpDB(env.enemyDB, "enemy", body, 0)
		table.sort(body)
		for _, l in ipairs(body) do lines[#lines + 1] = l end
		local f = openFile(outputPath .. ".pass" .. pass .. ".txt", "w")
		f:write(table.concat(lines, "\n"), "\n")
		f:close()
	end
	local ok, err = xpcall(function()
		calcs.buildOutput(build, "MAIN")
	end, traceback)
	calcs.perform = origPerform
	if not ok then
		fail("lua", "probe: " .. tostring(err))
		return false
	end
	return true
end
```

**Comparing the passes.** The first lines of the two dumps are `# mode=MAIN skipEHP=nil skill=Smite` and `# mode=CALCULATOR skipEHP=true skill=Smite`. Each dump without its first line and its `cond` line, with the level numbers removed (`skill0`, `player1`, ... to `skill`, `player`) and the `pout` and `eout` rows left out, sorted, then `diff`. (The `pout` rows were compared in an earlier run of the same probe: the only ones present in both passes and different are the damage outputs the `mh` and `out` rows show, and `CombinedDPS`, `CombinedAvg`, `AverageBurstDamage`, and the `With...DPS` rows that follow them; the full DPS pass has no defence outputs, as it skips them, `skipEHP`. The `cond` line differs only by `Buffed`, `Combat`, and `Effective`, which the full DPS pass's own ModDB does not hold; the mods that read them evaluate alike, as the `eval` rows show.) The whole diff for the Sand fixture:

```text
< eval Damage INC=0 MORE=0.824 BASE=0
> eval Damage INC=0 MORE=0.8208 BASE=0
< mh AverageDamage 69.722626875
< mh AverageHit 316.92103125
> mh AverageDamage 69.418463125
> mh AverageHit 315.53846875
< mh LightningCritAverage 432
< mh LightningHitAverage 288
> mh LightningCritAverage 430.125
> mh LightningHitAverage 286.75
< mh PhysicalCritAverage 40.78125
< mh PhysicalHitAverage 27.1875
> mh PhysicalCritAverage 40.59375
> mh PhysicalHitAverage 27.0625
< mh TotalDPS 96.008057206875
< mh TotalMax 548.75
< mh TotalMin 81.625
> mh TotalDPS 95.589223723125
> mh TotalMax 546.625
> mh TotalMin 81
< mh impaleStoredHitAvg 109.348125
> mh impaleStoredHitAvg 108.845375
< mult Level 90
< out AverageDamage 69.722626875
> out AverageDamage 69.418463125
< out TotalDPS 96.008057206875
> out TotalDPS 95.589223723125
```

The Blood fixture's diff has the same lines, with `eval Damage ... MORE=1.096` and `MORE=1.09728`, and `TotalDPS` 127.66806100125 and 127.76325042937. (`mult Level 90` is the multiplier table of the pass's own ModDB; in full DPS it is in the cached parent.)

**Where the modifiers are** (the dumps with their level numbers, `Damage` MORE rows other than the conditional ones):

```text
pass1 player0 Damage|MORE|-5|f512|k0|Skill:BloodAndSand|{effectCond=SandStance,effectName=Sand,effectType=Buff,type=GlobalEffect}{skillType=20,type=SkillType}
pass1 player0 Damage|MORE|4|f0|k0|Base|{type=Multiplier,var=FrenzyCharge}
pass2 player0 Damage|MORE|-5|f512|k0|Skill:BloodAndSand|{effectCond=SandStance,effectName=Sand,effectType=Buff,type=GlobalEffect}{skillType=20,type=SkillType}
pass2 player1 Damage|MORE|4|f0|k0|Base|{type=Multiplier,var=FrenzyCharge}
```

## Evidence

Run `local/p6/s4` (`b-ct`), the engine at `c180b5c7` (S4 changes no engine code); fixtures made by `local/p6/s4_fixtures.py`, the `pob-bug` entries by `local/p6/s4_pob_bug.py`:

- **Public `calc-parity`:** 567 builds, 0 unexpected, 718 expected differences; `Complete` 41,498 to 41,654 (the two fixtures). No value of another build moved (`move_cmp.py` against `local/p6/s3f`).
- **Corpus:** 0 unexpected; no value moved.
- **Runs:** public and corpus each twice, identical.
- **Tests:** `full_dps_should_add_the_skills_own_hit_dps`; the workspace tests (1,864), loom-calc's real-bundle tests (`b-ct`, 17), clippy, and rustfmt pass.
- **A guard fixed on the way:** the real-bundle test `real_exposure_and_shock_stats_should_be_covered` had failed since S3 part 7: "Trigger Level # Gore Shockwave on Melee Hit" names "shock" and is a granted-trigger row. It is now listed as such (`GRANTED_SKILL_ROWS`).

## What this corrects

P6-11eu (`docs/phase-6/stat-mop-up.md`) traced the difference to "PoB's shared environment" and named the stance buffs as likely carried over between skills. The run shows otherwise: `calcFullDPS` rebuilds the environment for every skill (`Calcs.lua` 335), nothing is carried over, and the difference is the ModDB levels' rounding above.

## Seen again: the Smite Inquisitor builds (the agent list, 2026-11-26)

With Static Strike's beams and Shockwave computed, the probe above, rebuilt in a scratchpad copy of the adapter from this page, ran on copies of `smite-inquisitor-act5.xml` and `act10.xml` with Static Strike's group as the main group. Rage's more damage, added during the calculation, and the frenzy charges' more damage (Base) round together in the main pass and apart in full DPS. act10: `Damage` MORE 0.70 x round(1.36 x 1.12, 2) = 1.064 against 0.70 x 1.36 x 1.12 = 1.06624. Registered `pob-bug` DIV-0780 (act10) and DIV-0782 (act5) (`docs/phase-6/triggers.md`, "Part 9").
