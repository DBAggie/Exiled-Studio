-- loom PoB data export (Phase 3 plan, step 2). Original code of this
-- project (MIT); not part of Path of Building. Maintainer-run, not in CI,
-- and outside the oracle adapter and its protocol (it reads no build).
--
-- Writes the game data PoB v2.67.2 holds after startup, for the values a
-- calculation reads, as one JSON document that `loom-parity-cli
-- data-parity` compares with a bundle:
--   luajit pob_data.lua <out.json> [tree=<version>]
-- run like oracle.lua (working directory <oracle-dir>/pob/src, LUA_PATH
-- and LUA_CPATH as `oracle-run` sets them; see tools/oracle/data/README.md).
-- `tree` defaults to PoB's latestTreeVersion.
--
-- PoB is started with its own HeadlessWrapper.lua, so every table is the
-- one PoB's calculations read (`data.*` from src/Modules/Data.lua and
-- src/Data/*). The tree is PoB's own TreeData/<version>/tree.lua as PoB
-- reads it, before PassiveTree.lua adds display lines. Only plain values
-- are copied: names, numbers, booleans, strings, and lists of them. No
-- value is converted; the comparison names each conversion PoB's exporter
-- made (tools/parity-cli/src/data_parity.rs). Writes go only to <out.json>.

---@diagnostic disable: undefined-global

local outputPath = ...
local treeArgument
for i = 2, select("#", ...) do
	local argument = select(i, ...)
	local value = argument:match("^tree=(.+)$")
	if value and not treeArgument then
		treeArgument = value
	else
		io.stderr:write("pob_data.lua: unknown argument ", argument, "\n")
		os.exit(1)
	end
end
local dataDir = (arg[0]:gsub("[^/\\]+$", ""))
local adapterDir = dataDir .. "../adapter/"

local function die(message)
	io.stderr:write("pob_data.lua: ", message, "\n")
	os.exit(1)
end

if not outputPath then
	die("usage: luajit pob_data.lua <out.json> [tree=<version>]")
end

local json = dofile(adapterDir .. "json.lua")
local shims = dofile(adapterDir .. "shims.lua")
local openFile = io.open
local refused = {}
io.open = shims.writeGuard(openFile, refused)

dofile("HeadlessWrapper.lua")
if launch.promptMsg then
	die("PoB startup: " .. tostring(launch.promptMsg))
end
if type(data) ~= "table" or type(data.skills) ~= "table" then
	die("PoB startup did not define data.skills")
end

local FORMAT = 12
local treeVersion = treeArgument or latestTreeVersion

-- A list that encodes as a JSON array even when empty is not possible with
-- the writer (an empty table is {}); the reader accepts {} for an empty list.
local function list(t)
	local out = {}
	for i, v in ipairs(t or {}) do
		out[i] = v
	end
	return out
end

-- Plain copy of the scalar fields of `t` named in `keys`.
local function pick(t, keys)
	local out = {}
	for _, key in ipairs(keys) do
		local v = t[key]
		local kind = type(v)
		if kind == "number" or kind == "string" or kind == "boolean" then
			out[key] = v
		end
	end
	return out
end

---------------------------------------------------------------------------
-- Skill types: PoB's names by number (src/Data/Global.lua `SkillType`).
---------------------------------------------------------------------------
local skillTypeNames = {}
local lastType = 0
for name, number in pairs(SkillType) do
	skillTypeNames[number] = name
	lastType = math.max(lastType, number)
end
local skillTypes = {}
for number = 1, lastType do
	skillTypes[number] = skillTypeNames[number] or ""
end

local function typeNames(set)
	local names = {}
	for number, on in pairs(set or {}) do
		if on then
			names[#names + 1] = skillTypeNames[number] or ("#" .. tostring(number))
		end
	end
	table.sort(names)
	return names
end

---------------------------------------------------------------------------
-- Skills (data.skills): the fields PoB's exporter writes from the game's
-- GrantedEffects, GrantedEffectsPerLevel, GrantedEffectStatSets(PerLevel),
-- GrantedEffectQualityStats (src/Export/Scripts/skills.lua).
---------------------------------------------------------------------------
-- Not copied: levelRequirement and Vaal soul costs (the bundle does not
-- carry them, P3-1a) and statInterpolation (kept raw until P3-9a).
local LEVEL_FIELDS = {
	"critChance", "damageEffectiveness", "baseMultiplier",
	"manaMultiplier", "attackSpeedMultiplier", "attackTime", "cooldown",
	"storedUses", "manaReservationFlat", "manaReservationPercent",
	"lifeReservationFlat", "lifeReservationPercent",
}

local function statPairs(rows)
	local out = {}
	for i, row in ipairs(rows or {}) do
		out[i] = { stat = row[1], value = row[2] }
	end
	return out
end

-- A mod's plain fields and its tags' plain fields (format 4, P3-12h). A
-- tag's `var`, `effectType`, `effectName`, and the like are what decide
-- whether a skill's stat reaches the character (`GlobalEffect`, see
-- Modules/CalcActiveSkill.lua 794 to 859).
local function modRecord(m)
	local out = pick(m, { "name", "type", "flags", "keywordFlags" })
	-- A map entry that is not a mod (PoB keeps a few helper tables there)
	-- is written as such, so the check sees it instead of skipping it.
	out.name = out.name or "<unnamed>"
	out.type = out.type or ""
	if type(m.value) == "number" or type(m.value) == "boolean" or type(m.value) == "string" then
		out.value = m.value
	elseif type(m.value) == "table" then
		out.value_fields = pick(m.value, { "key", "value", "type", "name" })
	end
	local tags = {}
	for i, tag in ipairs(m) do
		if type(tag) == "table" then
			tags[i] = pick(tag, { "type", "effectType", "effectName", "var", "varList", "actor", "skillType", "neg", "allowTotemBuff", "applyNotPlayer", "applyMinions", "unscalable", "effectCond", "effectEnemyCond", "div", "limit", "stat", "modCond", "effectStackVar", "effectStackLimit", "effectStackLimitVar", "skillName", "skillId", "threshold", "thresholdStat", "ifSkillType" })
		end
	end
	out.tags = tags
	return out
end

-- The mods each stat of a skill gives, as PoB's `mergeSkillInstanceMods`
-- (Modules/CalcTools.lua) finds them: the skill's own `statMap`, else
-- PoB's global `data.skillStatMap` (Data/SkillStatMap.lua); and the
-- skill's `baseMods` (stat "", for the mods every instance has). The
-- stats are the skill's level, constant, and quality stats and its
-- statMap keys. A stat neither map knows has no row.
local function skillMods(effect)
	local stats = {}
	local seen = {}
	local function add(stat)
		if type(stat) == "string" and not seen[stat] then
			seen[stat] = true
			stats[#stats + 1] = stat
		end
	end
	for _, s in ipairs(effect.stats or {}) do add(s) end
	for _, row in ipairs(effect.constantStats or {}) do add(row[1]) end
	for _, row in ipairs(effect.qualityStats or {}) do add(row[1]) end
	for s in pairs(effect.statMap or {}) do
		-- `_grantedEffect` is the back reference PoB sets (Data.lua), not a stat.
		if s ~= "_grantedEffect" then add(s) end
	end
	table.sort(stats)
	local out = {}
	for _, stat in ipairs(stats) do
		local map = (effect.statMap and effect.statMap[stat]) or data.skillStatMap[stat]
		if map then
			local mods = {}
			for _, m in ipairs(map) do
				if type(m) == "table" then
					mods[#mods + 1] = modRecord(m)
				end
			end
			out[#out + 1] = { stat = stat, own = (effect.statMap and effect.statMap[stat]) ~= nil, div = map.div, mult = map.mult, mods = mods }
		end
	end
	local base = {}
	for _, m in ipairs(effect.baseMods or {}) do
		if type(m) == "table" then
			base[#base + 1] = modRecord(m)
		end
	end
	out[#out + 1] = { stat = "", own = true, mods = base }
	return out
end

local skills = {}
for id, effect in pairs(data.skills) do
	local stats = list(effect.stats)
	local levels = {}
	for level, row in pairs(effect.levels or {}) do
		local out = pick(row, LEVEL_FIELDS)
		-- The level's values in `stats` order; stats past the last value
		-- (implicit stats) have none.
		out.values = list(row)
		local cost = {}
		for resource, amount in pairs(row.cost or {}) do
			if resource ~= "Soul" then
				cost[resource] = amount
			end
		end
		out.cost = cost
		levels[tostring(level)] = out
	end
	local skill = pick(effect, { "name", "castTime", "baseEffectiveness", "incrementalEffectiveness", "support", "color" })
	skill.stats = stats
	skill.types = typeNames(effect.skillTypes)
	skill.constant_stats = statPairs(effect.constantStats)
	skill.quality_stats = statPairs(effect.qualityStats)
	skill.levels = levels
	skill.mods = skillMods(effect)
	skill.minions = list(effect.minionList)
	local addTypes = {}
	for i, number in ipairs(effect.addSkillTypes or {}) do
		addTypes[i] = skillTypeNames[number] or ("#" .. tostring(number))
	end
	skill.add_types = addTypes
	skill.add_minions = list(effect.addMinionList)
	skill.is_trigger = effect.isTrigger == true
	-- PoB's hand-written skill data (format 10, Phase 6 plan decision 5):
	-- the skill's base flags, its parts with their plain fields (a part's
	-- flags, stages, and the like), and the names of its skill functions
	-- (ported by hand per skill).
	local flags = {}
	for name, on in pairs(effect.baseFlags or {}) do
		if on == true then flags[#flags + 1] = name end
	end
	table.sort(flags)
	skill.base_flags = flags
	local parts = {}
	for i, part in ipairs(effect.parts or {}) do
		local p = {}
		for k, v in pairs(part) do
			local kind = type(v)
			if kind == "number" or kind == "string" or kind == "boolean" then
				p[k] = v
			end
		end
		parts[i] = p
	end
	skill.parts = parts
	local functions = {}
	for k, v in pairs(effect) do
		if type(v) == "function" then functions[#functions + 1] = k end
	end
	table.sort(functions)
	skill.functions = functions
	skills[id] = skill
end

---------------------------------------------------------------------------
-- Gems (data.gems, src/Data/Gems.lua), keyed by PoB's key.
---------------------------------------------------------------------------
local gems = {}
for key, gem in pairs(data.gems) do
	local out = pick(gem, { "name", "gameId", "variantId", "grantedEffectId", "secondaryGrantedEffectId", "vaalGem", "reqStr", "reqDex", "reqInt", "naturalMaxLevel" })
	local tags = {}
	for tag, on in pairs(gem.tags or {}) do
		if on then
			tags[#tags + 1] = tag
		end
	end
	table.sort(tags)
	out.tags = tags
	gems[key] = out
end

---------------------------------------------------------------------------
-- Item bases (data.itemBases): weapon and armour properties.
---------------------------------------------------------------------------
local bases = {}
for name, base in pairs(data.itemBases) do
	if type(base.weapon) == "table" or type(base.armour) == "table" then
		bases[name] = {
			type = base.type,
			subType = base.subType,
			weapon = base.weapon and pick(base.weapon, { "PhysicalMin", "PhysicalMax", "CritChanceBase", "AttackRateBase", "Range" }) or nil,
			armour = base.armour and pick(base.armour, {
				"ArmourBaseMin", "ArmourBaseMax", "EvasionBaseMin", "EvasionBaseMax", "EnergyShieldBaseMin",
				"EnergyShieldBaseMax", "WardBaseMin", "WardBaseMax", "MovementPenalty", "BlockChance",
			}) or nil,
		}
	end
end

-- Every item base's requirements (data.itemBases[name].req, format 6, UX
-- sprint B-2): the level PoB's exporter derived and the attributes it
-- copied (src/Export/Scripts/bases.lua:276-305). `flask` says which level
-- rule the exporter used.
local baseRequirements = {}
for name, base in pairs(data.itemBases) do
	local req = type(base.req) == "table" and base.req or {}
	baseRequirements[name] = {
		type = base.type,
		flask = type(base.flask) == "table",
		level = req.level,
		str = req.str,
		dex = req.dex,
		int = req.int,
	}
end

-- Every flask base (data.itemBases[name].flask, format 11, Phase 6 P6-4b):
-- the recovery, duration, and buff lines PoB's exporter wrote
-- (src/Export/Scripts/bases.lua 247 to 270).
local flasks = {}
for name, base in pairs(data.itemBases) do
	if type(base.flask) == "table" then
		flasks[name] = {
			life = base.flask.life,
			mana = base.flask.mana,
			duration = base.flask.duration,
			buff = list(base.flask.buff),
		}
	end
end

-- Every minion and spectre (data.minions after Data.lua 1126 to 1133, format
-- 12, Phase 6 P6-7a): the values PoB's exporter wrote from each monster
-- variety (src/Export/Scripts/minions.lua), keyed by the variety's ID.
-- Spectres are keyed by it; a minion's key is a short name, which PoB's own
-- template (Export/Minions/Minions.txt, `#monster <id> <name>`) maps to
-- the variety. A name the template does not map is written under its key.
local minionVariety = {}
local template = openFile("Export/Minions/Minions.txt", "r")
if template then
	for line in template:lines() do
		local id, name = line:match("^#monster%s+(%S+)%s+(%S+)")
		if id and name then
			minionVariety[name] = id
		end
	end
	template:close()
end
local minions = {}
for key, m in pairs(data.minions) do
	local id = minionVariety[key] or key
	minions[id] = {
		key = key,
		name = m.name,
		life = m.life,
		energyShield = m.energyShield,
		armour = m.armour,
		evasion = m.evasion,
		fireResist = m.fireResist,
		coldResist = m.coldResist,
		lightningResist = m.lightningResist,
		chaosResist = m.chaosResist,
		damage = m.damage,
		damageSpread = m.damageSpread,
		attackTime = m.attackTime,
		attackRange = m.attackRange,
		accuracy = m.accuracy,
		damageFixup = m.damageFixup,
		lifeScaling = m.lifeScaling,
		baseDamageIgnoresAttackSpeed = m.baseDamageIgnoresAttackSpeed == true,
		weaponType1 = m.weaponType1,
		weaponType2 = m.weaponType2,
		monsterTags = list(m.monsterTags),
		skillList = list(m.skillList),
	}
end

---------------------------------------------------------------------------
-- Item mods (format 6, UX sprint B-2): the affix name and level of each
-- mod PoB keys by the game's mod ID, from the files Modules/Data.lua loads
-- into data.itemMods (`Prefix:`/`Suffix:` spec lines name these keys,
-- Classes/Item.lua 1008-1010). `Item` is a merge of the first seven and
-- `WatchersEye` is derived, so neither is read; data.masterMods has no
-- IDs. `tables` lists the files holding the ID, in this order; `conflict`
-- is set when two of them disagree on the affix or level.
---------------------------------------------------------------------------
local MOD_TABLES = {
	"Explicit", "ItemExclusive", "Corrupted", "Delve", "Synthesis", "Scourge", "Eldritch",
	"Flask", "Tincture", "Graft", "Jewel", "JewelAbyss", "JewelCluster", "JewelCharm", "Foulborn",
}
local itemMods = {}
for _, tableName in ipairs(MOD_TABLES) do
	local source = data.itemMods[tableName]
	if type(source) ~= "table" then
		die("data.itemMods." .. tableName .. " is missing")
	end
	for id, mod in pairs(source) do
		if type(id) == "string" and type(mod) == "table" then
			local affix = type(mod.affix) == "string" and mod.affix or ""
			local record = itemMods[id]
			if not record then
				itemMods[id] = { affix = affix, level = mod.level, type = mod.type, tables = { tableName } }
			else
				record.tables[#record.tables + 1] = tableName
				if record.affix ~= affix or record.level ~= mod.level then
					record.conflict = true
				end
			end
		end
	end
end

---------------------------------------------------------------------------
-- Monster tables and constants (src/Data/Misc.lua).
---------------------------------------------------------------------------
local MONSTER_TABLES = {
	"monsterEvasionTable", "monsterAccuracyTable", "monsterLifeTable", "monsterLifeTable2",
	"monsterLifeTable3", "monsterAllyLifeTable", "monsterDamageTable", "monsterAllyDamageTable",
	"monsterArmourTable", "monsterAilmentThresholdTable", "monsterPhysConversionMultiTable",
}
local monsterTables = {}
for _, name in ipairs(MONSTER_TABLES) do
	monsterTables[name] = list(data[name])
end

-- PoB's bosses (`data.bosses`, `Data/Bosses.lua`): the multipliers its boss
-- enemies' armour and evasion average (format 2, P3-1d).
local bosses = {}
for name, boss in pairs(data.bosses or {}) do
	bosses[name] = { armourMult = boss.armourMult, evasionMult = boss.evasionMult, isUber = boss.isUber == true }
end

-- PoB's pantheon (data.pantheons, Data/Pantheons.lua): each god's major
-- mark and its souls, each soul's name and its mods' lines and values, in
-- PoB's order (format 3, P3-12g).
local pantheons = {}
for id, god in pairs(data.pantheons or {}) do
	local souls = {}
	for i, soul in ipairs(god.souls or {}) do
		local mods = {}
		for j, mod in ipairs(soul.mods or {}) do
			mods[j] = { line = mod.line, value = list(mod.value) }
		end
		souls[i] = { name = soul.name, mods = mods }
	end
	pantheons[id] = { isMajorGod = god.isMajorGod == true, souls = souls }
end

local function copyMap(t)
	local out = {}
	for k, v in pairs(t or {}) do
		if type(v) == "number" then
			out[k] = v
		end
	end
	return out
end

---------------------------------------------------------------------------
-- The tree as PoB reads it (TreeData/<version>/tree.lua).
---------------------------------------------------------------------------
local treeFile = "TreeData/" .. treeVersion .. "/tree.lua"
local treeChunk, treeError = loadfile(treeFile)
if not treeChunk then
	die("cannot load " .. treeFile .. ": " .. tostring(treeError))
end
local rawTree = treeChunk()

local classes = {}
for i, class in ipairs(rawTree.classes or {}) do
	local ascendancies = {}
	for j, ascendancy in ipairs(class.ascendancies or {}) do
		ascendancies[j] = ascendancy.id or ascendancy.name
	end
	classes[i] = {
		name = class.name,
		base_str = class.base_str,
		base_dex = class.base_dex,
		base_int = class.base_int,
		ascendancies = ascendancies,
	}
end

local NODE_FIELDS = {
	"skill", "name", "isKeystone", "isNotable", "isMastery", "isJewelSocket", "ascendancyName",
	"isAscendancyStart", "classStartIndex", "isMultipleChoice", "isMultipleChoiceOption",
	"isBlighted", "isProxy", "isBloodline", "grantedPassivePoints", "grantedStrength",
	"grantedDexterity", "grantedIntelligence",
}
local nodes = {}
for key, node in pairs(rawTree.nodes or {}) do
	if type(node) == "table" and node.skill then
		local out = pick(node, NODE_FIELDS)
		out.stats = list(node.stats)
		out.expansionJewel = node.expansionJewel ~= nil
		local effects = {}
		for i, effect in ipairs(node.masteryEffects or {}) do
			effects[i] = { effect = effect.effect, stats = list(effect.stats) }
		end
		out.masteryEffects = effects
		nodes[tostring(key)] = out
	end
end

---------------------------------------------------------------------------
-- Jewel radii (format 7, passive tree plan T-f): data.jewelRadius, the
-- latest tree version's list (Modules/Data.lua 597-624), in PoB's order:
-- the full radii, then the rings ("Variable").
---------------------------------------------------------------------------
local jewelRadii = {}
for i, r in ipairs(data.jewelRadius or {}) do
	jewelRadii[i] = { inner = r.inner, outer = r.outer, label = r.label }
end

---------------------------------------------------------------------------
-- Unique flavour text (format 8, B-2): data.flavourText (src/Data/
-- FlavourText.lua, written by src/Export/Scripts/flavourText.lua), in PoB's
-- order: each entry's FlavourText ID, the unique's name, and its lines as
-- PoB's exporter cleaned them.
---------------------------------------------------------------------------
local flavourText = {}
for i, entry in ipairs(data.flavourText or {}) do
	flavourText[i] = { id = entry.id, name = entry.name, text = list(entry.text) }
end

---------------------------------------------------------------------------
-- Cluster jewels (format 9, passive tree plan T-i): data.clusterJewels
-- (src/Data/ClusterJewels.lua, written by src/Export/Scripts/cluster.lua):
-- each jewel by its base's name with its layout and its small passives'
-- tags (by passive ID), the notables' sort order (by name), the keystones'
-- names, and each proxy's start indices by size.
---------------------------------------------------------------------------
local clusterData = data.clusterJewels or {}
local clusterJewels = {
	jewels = {},
	notable_sort_order = copyMap(clusterData.notableSortOrder),
	keystones = list(clusterData.keystones),
	orbit_offsets = {},
}
for name, jewel in pairs(clusterData.jewels or {}) do
	local skills = {}
	for id, skill in pairs(jewel.skills or {}) do
		skills[id] = { name = skill.name, tag = skill.tag }
	end
	clusterJewels.jewels[name] = {
		size_index = jewel.sizeIndex,
		min_nodes = jewel.minNodes,
		max_nodes = jewel.maxNodes,
		small = list(jewel.smallIndicies),
		notable = list(jewel.notableIndicies),
		socket = list(jewel.socketIndicies),
		total = jewel.totalIndicies,
		skills = skills,
	}
end
for proxy, offsets in pairs(clusterData.orbitOffsets or {}) do
	local starts = {}
	for size = 0, 2 do
		if offsets[size] == nil then
			break
		end
		starts[size + 1] = offsets[size]
	end
	clusterJewels.orbit_offsets[tostring(proxy)] = starts
end

---------------------------------------------------------------------------
-- The document.
---------------------------------------------------------------------------
local document = {
	format = FORMAT,
	pob = { version = launch.versionNumber, branch = launch.versionBranch, runtime = jit and jit.version },
	tree_version = treeVersion,
	skill_types = skillTypes,
	skills = skills,
	gems = gems,
	bases = bases,
	base_requirements = baseRequirements,
	flasks = flasks,
	minions = minions,
	mods = itemMods,
	monster_tables = monsterTables,
	game_constants = copyMap(data.gameConstants),
	character_constants = copyMap(data.characterConstants),
	monster_constants = copyMap(data.monsterConstants),
	bosses = bosses,
	pantheons = pantheons,
	jewel_radii = list(jewelRadii),
	flavour_text = list(flavourText),
	cluster_jewels = clusterJewels,
	tree = { classes = classes, nodes = nodes },
}

if #refused > 0 then
	die("PoB tried to write " .. refused[1])
end

local ok, text = pcall(json.encode, document, "document")
if not ok then
	if type(text) == "table" then
		die("JSON: " .. tostring(text.path) .. ": " .. tostring(text.message))
	end
	die("JSON: " .. tostring(text))
end
local file, err = openFile(outputPath, "wb")
if not file then
	die("cannot write the output: " .. tostring(err))
end
file:write(text, "\n")
file:close()
