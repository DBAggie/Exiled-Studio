-- loom oracle adapter, protocol 4 (Spike C, dev/completed/spike-c-oracle-harness/;
-- protocol 4: Phase 3 P3-5, docs/phase-3/calc-parity.md).
-- Original code of this project (MIT); not part of Path of Building.
--
-- Run by `loom-parity-cli oracle-run` with the pinned LuaJIT, the working
-- directory at <oracle-dir>/pob/src, and a cleared environment:
--   luajit oracle.lua <fixture.xml> <output.json> [breakdown=<name>...] [opaque=<key>...] [zlib=<library>]
-- Drives PoB's own HeadlessWrapper.lua: loads the fixture with the
-- wrapper's loadBuildFromXML (the read path of PoB's spec/GenerateBuilds.lua)
-- and writes exactly one JSON document to <output.json>, never to stdout,
-- so PoB's prints cannot corrupt the protocol. The adapter reports facts
-- (stage reached, error text); the CLI assigns failure classes.
--
-- Exit codes: 0 when the document says "done", 1 when it records a
-- failure, 2 when no document could be written.
--
-- The adapter patches no PoB file. It replaces three globals with named
-- shims (shims.lua: `Inflate`, `pairs` for ambiguous gem variant tables,
-- and `io.open`), records them and what they did in the document, and
-- records which modules `require` loads. The output contract is
-- tools/oracle/schema/oracle-output.schema.json.
--
-- Protocol 4: after loading, the adapter sets PoB's CALCS view (which
-- the breakdowns come from) to the main view (which main_output comes
-- from), so both describe the main skill; see alignCalcsView.

-- launch, build, common, loadBuildFromXML are PoB's globals.
---@diagnostic disable: undefined-global

local PROTOCOL = 4
local BREAKDOWN_SOURCE = "calcsTab.calcsEnv.player.breakdown"

-- PoB's startup replaces the global `arg`, so the arguments (and arg[0],
-- this file's path) are read first.
local fixturePath, outputPath = ...
local breakdownNames, opaqueKeys, badArguments = {}, {}, {}
local zlibLibrary
for i = 3, select("#", ...) do
	local argument = select(i, ...)
	local kind, value = argument:match("^(%a+)=(.+)$")
	if kind == "breakdown" then
		breakdownNames[#breakdownNames + 1] = value
	elseif kind == "opaque" then
		opaqueKeys[value] = true
	elseif kind == "zlib" and zlibLibrary == nil then
		zlibLibrary = value
	else
		badArguments[#badArguments + 1] = argument
	end
end
local adapterDir = (arg[0]:gsub("[^/\\]+$", ""))

local result = {
	protocol = PROTOCOL,
	stage = "startup",
	error = nil,
	partial = {},
	modules = {},
	settings = nil,
	main_output = nil,
	breakdowns = nil,
	breakdown_source = BREAKDOWN_SOURCE,
	-- What the shims did: every Inflate call, every refused write, every
	-- gem variant table iterated in sorted order (by gameId, once each).
	probe = { inflate = {}, writes_refused = {}, variant_order = {}, calcs_view = {} },
}

local partialSeen = {}
local function addPartial(reason)
	if not partialSeen[reason] then
		partialSeen[reason] = true
		result.partial[#result.partial + 1] = reason
	end
end

-- Module log: the wrapper wraps the global `require` it finds, so every
-- module PoB loads passes through here (the wrapper answers `lcurl.safe`
-- itself, without loading anything).
local loaded = {}
local baseRequire = require
require = function(name)
	loaded[name] = true
	return baseRequire(name)
end

-- The JSON writer and the shims, next to this file.
local json = dofile(adapterDir .. "json.lua")
local NormalizationError = json.NormalizationError
local formatString = json.string
local shims = dofile(adapterDir .. "shims.lua")

---------------------------------------------------------------------------
-- Shims (named, recorded in every document; see shims.lua)
---------------------------------------------------------------------------

-- The adapter's own file access keeps the original io.open.
local openFile = io.open
io.open = shims.writeGuard(openFile, result.probe.writes_refused)

local zlib, zlibError = shims.loadZlib(zlibLibrary)

-- variant_order: installed before PoB starts, since PoB's files copy the
-- global `pairs` into locals when they load. Tables are registered after
-- startup, when Data.lua has built data.gemsByGameId.
local basePairs = pairs
local orderedTables = setmetatable({}, { __mode = "k" })
local orderedSeen = {}
local variantTables
pairs = shims.orderedPairs(basePairs, orderedTables, function(gameId)
	if not orderedSeen[gameId] then
		orderedSeen[gameId] = true
		result.probe.variant_order[#result.probe.variant_order + 1] = gameId
	end
end, function(gameId)
	-- Cannot be sorted, so PoB's order (which changes per run) decides.
	addPartial("variant_order: unordered table " .. gameId)
end)

-- Replaces the wrapper's Inflate stub. Each call is recorded in the probe.
-- Without zlib it returns what the stub returns (""), and the result is
-- partial; a zlib error returns nil (PoB's loader then has no data), and
-- the result is partial.
local function inflateShim(data)
	local call = { input_bytes = type(data) == "string" and #data or 0 }
	result.probe.inflate[#result.probe.inflate + 1] = call
	if not zlib then
		call.error = zlibError
		addPartial("inflate: " .. zlibError)
		return ""
	end
	local inflated, adlerOrError, trailing = shims.inflate(zlib, data)
	if not inflated then
		call.error = adlerOrError
		addPartial("inflate: " .. adlerOrError)
		return nil
	end
	call.output_bytes, call.adler32, call.trailing_bytes = #inflated, adlerOrError, trailing
	return inflated
end

-- The document's shim records, by name.
local function shimRecords()
	return {
		{ name = "inflate", replaces = "Inflate", installed = zlib ~= nil, detail = zlib and ("zlib " .. zlib.version) or zlibError },
		{
			name = "variant_order",
			replaces = "pairs",
			installed = variantTables ~= nil,
			detail = variantTables and (variantTables .. " ambiguous gem variant tables") or "data.gemsByGameId not found",
		},
		{ name = "write_guard", replaces = "io.open", installed = true },
	}
end

---------------------------------------------------------------------------
-- Stages
---------------------------------------------------------------------------

local function traceback(err)
	if getmetatable(err) == NormalizationError then
		return err
	end
	return debug.traceback(tostring(err), 2)
end

local function fail(source, message)
	result.error = { source = source, message = message }
end

-- The prompt PoB shows after an error (`launch:ShowErrMsg`), if any.
local function promptMessage()
	return type(launch) == "table" and launch.promptMsg or nil
end

local function startup()
	if #badArguments > 0 then
		fail("adapter", "unknown argument " .. badArguments[1])
		return false
	end
	local ok, err = xpcall(function()
		dofile("HeadlessWrapper.lua")
	end, traceback)
	-- The wrapper defines the Inflate stub before it starts PoB; jewel data
	-- loads only with a build, so replacing it now is in time.
	Inflate = inflateShim
	if not ok then
		fail("lua", tostring(err))
		return false
	end
	-- On a startup error the wrapper prints the prompt, reads stdin (closed,
	-- so it returns at once), and returns before setting `build`.
	if promptMessage() then
		fail("prompt", promptMessage())
		return false
	end
	if type(build) ~= "table" or type(loadBuildFromXML) ~= "function" then
		fail("adapter", "the wrapper did not define build and loadBuildFromXML")
		return false
	end
	if type(data) == "table" and type(data.gemsByGameId) == "table" then
		variantTables = shims.registerAmbiguousVariants(data.gemsByGameId, orderedTables, basePairs)
	else
		-- Without the table, an old-format Vaal gem may load a different
		-- variant per run: the result cannot be trusted as complete.
		addPartial("variant_order: data.gemsByGameId not found")
	end
	-- GetUserPath() returns "" headless, so PoB looks for its settings at
	-- "<userPath>Settings.xml". A run must not depend on a file there.
	local userPath = launch.main and launch.main.userPath
	if type(userPath) == "string" then
		local path = userPath .. "Settings.xml"
		local file = openFile(path, "rb")
		if file then
			file:close()
		end
		result.settings = { path = path, found = file ~= nil }
	end
	return true
end

local xmlText

local function import()
	if type(fixturePath) ~= "string" then
		fail("adapter", "no fixture path given")
		return false
	end
	local file, err = openFile(fixturePath, "rb")
	if not file then
		fail("adapter", "cannot open the fixture: " .. tostring(err))
		return false
	end
	xmlText = file:read("*a")
	file:close()
	-- The checks PoB's buildMode:LoadDB makes before it reads any section,
	-- with PoB's own parser; LoadDB only shows a prompt for them.
	local ok, parsed, parseError = xpcall(function()
		return common.xml.ParseXML(xmlText)
	end, traceback)
	if not ok then
		fail("lua", tostring(parsed))
		return false
	elseif parseError then
		fail("pob", tostring(parseError))
		return false
	elseif type(parsed) ~= "table" or type(parsed[1]) ~= "table" or parsed[1].elem ~= "PathOfBuilding" then
		fail("pob", "'PathOfBuilding' root element missing")
		return false
	end
	return true
end

-- The breakdowns come from PoB's CALCS view (calcsTab.calcsEnv), which
-- calculates the Calcs tab's socket group (the Calcs section's Input
-- skill_number) with that group's mainActiveSkillCalcs and its gems'
-- skillPartCalcs, skillStageCountCalcs, skillMineCountCalcs,
-- skillMinionCalcs, skillMinionItemSetCalcs, skillMinionSkillCalcs;
-- main_output comes from the main view (Build mainSocketGroup,
-- mainActiveSkill, and the gems' attributes without "Calcs").
-- Modules/CalcSetup.lua, Modules/CalcActiveSkill.lua, Classes/SkillsTab.lua
-- (LoadSkill), Classes/CalcsTab.lua (Load) at the pinned tag. A build file
-- keeps the Calcs tab's own selection, so the breakdowns can describe
-- another skill than main_output (P3-5: 16 of 36 public fixtures).
--
-- Before PoB reads the build, the adapter copies each main-view value to
-- its CALCS twin in the parsed XML and composes it again with PoB's own
-- xml.ComposeXML (the function PoB saves builds with). PoB then loads and
-- calculates once, in its usual order, as if the user had selected the
-- main skill in the Calcs tab. Recalculating after loading is not an
-- option: PoB's passes are not repeatable in one process (a second pass
-- changed main_output in 11 and the CALCS view's Life in 7 public
-- fixtures, P3-5). When no value differs (PoB reads an unset socket
-- group, active skill, or skill part as 1), the original text is loaded
-- unchanged. Each difference is recorded in probe.calcs_view.
local GEM_CALCS = { "skillPart", "skillStageCount", "skillMineCount", "skillMinion", "skillMinionItemSet", "skillMinionSkill" }
local DEFAULT_ONE = { skillPart = true, mainActiveSkill = true, skill_number = true }

local function alignCalcsView(text)
	local changes = result.probe.calcs_view
	local root = common.xml.ParseXML(text)[1]
	local function children(node, elem)
		local list = {}
		for _, c in ipairs(node) do
			if type(c) == "table" and c.elem == elem then
				list[#list + 1] = c
			end
		end
		return list
	end
	-- The value PoB uses for an attribute (an unset one is 1 where PoB
	-- defaults or clamps it from 1).
	local function effective(name, value)
		if DEFAULT_ONE[name] then
			return tonumber(value) or 1
		end
		return value
	end
	local function copy(attrib, name, value, label)
		local twin = name .. "Calcs"
		if effective(name, attrib[twin]) ~= effective(name, value) then
			changes[#changes + 1] = label .. " " .. twin .. " " .. tostring(attrib[twin]) .. " -> " .. tostring(value)
		end
		attrib[twin] = value
	end
	local build = children(root, "Build")[1]
	local main = build and (tonumber(build.attrib.mainSkillIndex) or tonumber(build.attrib.mainSocketGroup)) or 1
	local calcs = children(root, "Calcs")[1]
	if not calcs then
		calcs = { elem = "Calcs", attrib = {} }
		root[#root + 1] = calcs
	end
	local input
	for _, node in ipairs(children(calcs, "Input")) do
		if node.attrib.name == "skill_number" then
			input = node
		end
	end
	if not input then
		input = { elem = "Input", attrib = { name = "skill_number" } }
		table.insert(calcs, 1, input)
	end
	if effective("skill_number", input.attrib.number) ~= main then
		changes[#changes + 1] = "skill_number " .. tostring(input.attrib.number) .. " -> " .. main
	end
	input.attrib.number, input.attrib.string, input.attrib.boolean = tostring(main), nil, nil
	-- Both views calculate every group of the active skill set (the other
	-- groups' parts, stages, and minions give buffs, auras, and the like;
	-- Modules/CalcActiveSkill.lua reads the CALCS twins in CALCS mode), so
	-- every group of that set is copied; other sets are not calculated.
	local skills = children(root, "Skills")[1]
	local groups = {}
	if skills then
		local sets = children(skills, "SkillSet")
		if #sets == 0 then
			groups = children(skills, "Skill")
		else
			local active = skills.attrib.activeSkillSet or "1"
			for _, set in ipairs(sets) do
				if set.attrib.id == active then
					groups = children(set, "Skill")
				end
			end
		end
	end
	for index, skill in ipairs(groups) do
		local label = "group " .. index
		copy(skill.attrib, "mainActiveSkill", skill.attrib.mainActiveSkill, label)
		for j, gem in ipairs(children(skill, "Gem")) do
			for _, name in ipairs(GEM_CALCS) do
				local value = gem.attrib[name]
				-- An old-format group's skillPart is its first gem's
				-- (SkillsTab:LoadSkill).
				if name == "skillPart" and j == 1 and skill.attrib.skillPart then
					value = skill.attrib.skillPart
				end
				copy(gem.attrib, name, value, label .. " gem " .. j)
			end
		end
	end
	if #changes == 0 then
		return text
	end
	return common.xml.ComposeXML(root)
end

local function calculate()
	local ok, err = xpcall(function()
		loadBuildFromXML(alignCalcsView(xmlText), "fixture")
	end, traceback)
	if not ok then
		fail("lua", tostring(err))
		return false
	end
	-- Errors inside PoB's frame are caught by launch:OnFrame and shown as a
	-- prompt.
	if promptMessage() then
		fail("prompt", promptMessage())
		return false
	end
	if type(build.calcsTab) ~= "table" or type(build.calcsTab.mainOutput) ~= "table" then
		fail("adapter", "build.calcsTab.mainOutput is not a table after loading")
		return false
	end
	return true
end

local function normalize()
	local ok, err = xpcall(function()
		result.main_output = json.encode(build.calcsTab.mainOutput, "main_output", opaqueKeys)
		local env = build.calcsTab.calcsEnv
		local source = type(env) == "table" and type(env.player) == "table" and env.player.breakdown or nil
		local parts = {}
		for _, name in ipairs(breakdownNames) do
			local value = type(source) == "table" and source[name] or nil
			parts[#parts + 1] = formatString(name, "breakdowns") .. ":" .. json.encode(value, "breakdowns." .. name, opaqueKeys)
		end
		table.sort(parts)
		result.breakdowns = "{" .. table.concat(parts, ",") .. "}"
	end, traceback)
	if not ok then
		if getmetatable(err) == NormalizationError then
			fail("adapter", err.path .. ": " .. err.message)
		else
			fail("lua", tostring(err))
		end
		result.main_output, result.breakdowns = nil, nil
		return false
	end
	return true
end

local stages = { { "startup", startup }, { "import", import }, { "calculate", calculate }, { "normalize", normalize } }
local done = true
for _, stage in ipairs(stages) do
	result.stage = stage[1]
	if not stage[2]() then
		done = false
		break
	end
end
if done then
	result.stage = "done"
end

for name in pairs(loaded) do
	result.modules[#result.modules + 1] = name
end
table.sort(result.modules)

-- The only write a headless run is known to try is the jewel loader's
-- .bin cache, which PoB never reads back in the same run (it reads a .bin
-- only when NewFileSearch, stubbed, reports one). Any other refused write
-- might have been read back, so the result is partial; the paths are in
-- the probe.
for _, path in ipairs(result.probe.writes_refused) do
	if not path:find("Data/TimelessJewelData/[^/\\]+%.bin$") then
		addPartial("write_guard: refused a write PoB may read back")
	end
end

---------------------------------------------------------------------------
-- The document. main_output and breakdowns are already JSON text.
---------------------------------------------------------------------------

local function stringList(list)
	local parts = {}
	for i, s in ipairs(list) do
		parts[i] = formatString(s, "list")
	end
	return "[" .. table.concat(parts, ",") .. "]"
end

local function stringOrNull(value)
	return value == nil and "null" or formatString(value, "string")
end

local function integerOrNull(value)
	return value == nil and "null" or string.format("%d", value)
end

local function shimsJson()
	local parts = {}
	for i, shim in ipairs(shimRecords()) do
		parts[i] = '{"name":' .. formatString(shim.name, "shims") .. ',"replaces":' .. formatString(shim.replaces, "shims")
			.. ',"installed":' .. tostring(shim.installed) .. ',"detail":' .. stringOrNull(shim.detail) .. "}"
	end
	return "[" .. table.concat(parts, ",") .. "]"
end

local function probeJson()
	local calls = {}
	for i, call in ipairs(result.probe.inflate) do
		calls[i] = '{"input_bytes":' .. integerOrNull(call.input_bytes) .. ',"output_bytes":' .. integerOrNull(call.output_bytes)
			.. ',"adler32":' .. integerOrNull(call.adler32) .. ',"trailing_bytes":' .. integerOrNull(call.trailing_bytes)
			.. ',"error":' .. stringOrNull(call.error) .. "}"
	end
	return '{"inflate":[' .. table.concat(calls, ",") .. '],"variant_order":' .. stringList(result.probe.variant_order)
		.. ',"writes_refused":' .. stringList(result.probe.writes_refused)
		.. ',"calcs_view":' .. stringList(result.probe.calcs_view) .. "}"
end

local function document()
	local fields = {
		'"protocol":' .. PROTOCOL,
		'"stage":' .. formatString(result.stage, "stage"),
		'"error":' .. (result.error and ('{"source":' .. formatString(result.error.source, "error") .. ',"message":' .. formatString(result.error.message, "error") .. "}") or "null"),
		'"partial":' .. stringList(result.partial),
		'"modules":' .. stringList(result.modules),
		'"settings":' .. (result.settings and ('{"path":' .. formatString(result.settings.path, "settings") .. ',"found":' .. tostring(result.settings.found) .. "}") or "null"),
		'"shims":' .. shimsJson(),
		'"probe":' .. probeJson(),
		'"breakdown_source":' .. formatString(result.breakdown_source, "breakdown_source"),
		'"breakdowns":' .. (result.breakdowns or "null"),
		'"main_output":' .. (result.main_output or "null"),
	}
	return "{" .. table.concat(fields, ",") .. "}\n"
end

local ok, text = pcall(document)
if not ok then
	-- A string that is not valid UTF-8 outside the data trees (an error
	-- message, the settings path, a module name, a refused path): report
	-- what is known.
	result.error = { source = "adapter", message = "error text is not valid UTF-8" }
	result.main_output, result.breakdowns = nil, nil
	if result.stage == "done" then
		result.stage = "normalize"
	end
	result.settings = result.settings and { path = "(not valid UTF-8)", found = result.settings.found }
	result.modules = {}
	for i, path in ipairs(result.probe.writes_refused) do
		result.probe.writes_refused[i] = (path:gsub("[\128-\255]", "?"))
	end
	for _, call in ipairs(result.probe.inflate) do
		call.error = call.error and "(not valid UTF-8)"
	end
	for i, gameId in ipairs(result.probe.variant_order) do
		result.probe.variant_order[i] = (gameId:gsub("[\128-\255]", "?"))
	end
	for i, reason in ipairs(result.partial) do
		result.partial[i] = (reason:gsub("[\128-\255]", "?"))
	end
	done = false
	ok, text = pcall(document)
	if not ok then
		io.stderr:write("oracle.lua: cannot write the document: ", tostring(text), "\n")
		os.exit(2)
	end
end

local file = type(outputPath) == "string" and openFile(outputPath, "wb")
if not file then
	io.stderr:write("oracle.lua: cannot open the output file\n")
	os.exit(2)
end
file:write(text)
file:close()
os.exit(done and 0 or 1)
