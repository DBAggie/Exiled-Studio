-- loom PoB data export (Phase 3 plan, step 2). Original code of this
-- project (MIT); not part of Path of Building. Maintainer-run, not in CI,
-- and outside the oracle adapter and its protocol (protocol 3 unchanged).
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

local FORMAT = 1
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
for name, number in pairs(SkillType) do
	skillTypeNames[number] = name
end
local skillTypes = {}
for number = 1, #skillTypeNames do
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
	local skill = pick(effect, { "name", "castTime", "baseEffectiveness", "incrementalEffectiveness", "support" })
	skill.stats = stats
	skill.types = typeNames(effect.skillTypes)
	skill.constant_stats = statPairs(effect.constantStats)
	skill.quality_stats = statPairs(effect.qualityStats)
	skill.levels = levels
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
-- The document.
---------------------------------------------------------------------------
local document = {
	format = FORMAT,
	pob = { version = launch.versionNumber, branch = launch.versionBranch },
	tree_version = treeVersion,
	skill_types = skillTypes,
	skills = skills,
	gems = gems,
	bases = bases,
	monster_tables = monsterTables,
	game_constants = copyMap(data.gameConstants),
	character_constants = copyMap(data.characterConstants),
	monster_constants = copyMap(data.monsterConstants),
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
