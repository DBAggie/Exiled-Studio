-- loom oracle adapter, protocol 1 (Spike C, dev/active/spike-c-oracle-harness/).
-- Original code of this project (MIT); not part of Path of Building.
--
-- Run by `loom-parity-cli oracle-run` with the pinned LuaJIT, the working
-- directory at <oracle-dir>/pob/src, and a cleared environment:
--   luajit oracle.lua <fixture.xml> <output.json> [breakdown=<name>...] [opaque=<key>...]
-- Drives PoB's own HeadlessWrapper.lua: loads the fixture with the
-- wrapper's loadBuildFromXML (the read path of PoB's spec/GenerateBuilds.lua)
-- and writes exactly one JSON document to <output.json>, never to stdout,
-- so PoB's prints cannot corrupt the protocol. The adapter reports facts
-- (stage reached, error text); the CLI assigns failure classes.
--
-- Exit codes: 0 when the document says "done", 1 when it records a
-- failure, 2 when no document could be written.
--
-- The adapter patches no PoB file and replaces no PoB function; it only
-- records which modules `require` loads. The output contract is
-- tools/oracle/schema/oracle-output.schema.json.

-- launch, build, common, loadBuildFromXML are PoB's globals.
---@diagnostic disable: undefined-global

local PROTOCOL = 1
local BREAKDOWN_SOURCE = "calcsTab.calcsEnv.player.breakdown"

-- PoB's startup replaces the global `arg`, so the arguments (and arg[0],
-- this file's path) are read first.
local fixturePath, outputPath = ...
local breakdownNames, opaqueKeys, badArguments = {}, {}, {}
for i = 3, select("#", ...) do
	local argument = select(i, ...)
	local kind, value = argument:match("^(%a+)=(.+)$")
	if kind == "breakdown" then
		breakdownNames[#breakdownNames + 1] = value
	elseif kind == "opaque" then
		opaqueKeys[value] = true
	else
		badArguments[#badArguments + 1] = argument
	end
end

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
}

-- Module log: the wrapper wraps the global `require` it finds, so every
-- module PoB loads passes through here (the wrapper answers `lcurl.safe`
-- itself, without loading anything).
local loaded = {}
local baseRequire = require
require = function(name)
	loaded[name] = true
	return baseRequire(name)
end

-- The JSON writer (tools/oracle/adapter/json.lua), next to this file.
local json = dofile((arg[0]:gsub("[^/\\]+$", "")) .. "json.lua")
local NormalizationError = json.NormalizationError
local formatString = json.string

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
	-- GetUserPath() returns "" headless, so PoB looks for its settings at
	-- "<userPath>Settings.xml". A run must not depend on a file there.
	local userPath = launch.main and launch.main.userPath
	if type(userPath) == "string" then
		local path = userPath .. "Settings.xml"
		local file = io.open(path, "rb")
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
	local file, err = io.open(fixturePath, "rb")
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

local function calculate()
	local ok, err = xpcall(function()
		loadBuildFromXML(xmlText, "fixture")
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

local function document()
	local fields = {
		'"protocol":' .. PROTOCOL,
		'"stage":' .. formatString(result.stage, "stage"),
		'"error":' .. (result.error and ('{"source":' .. formatString(result.error.source, "error") .. ',"message":' .. formatString(result.error.message, "error") .. "}") or "null"),
		'"partial":' .. stringList(result.partial),
		'"modules":' .. stringList(result.modules),
		'"settings":' .. (result.settings and ('{"path":' .. formatString(result.settings.path, "settings") .. ',"found":' .. tostring(result.settings.found) .. "}") or "null"),
		'"breakdown_source":' .. formatString(result.breakdown_source, "breakdown_source"),
		'"breakdowns":' .. (result.breakdowns or "null"),
		'"main_output":' .. (result.main_output or "null"),
	}
	return "{" .. table.concat(fields, ",") .. "}\n"
end

local ok, text = pcall(document)
if not ok then
	-- A string that is not valid UTF-8 outside the data trees (an error
-- message, the settings path, a module name): report what is known.
	result.error = { source = "adapter", message = "error text is not valid UTF-8" }
	result.main_output, result.breakdowns = nil, nil
	if result.stage == "done" then
		result.stage = "normalize"
	end
	result.settings = result.settings and { path = "(not valid UTF-8)", found = result.settings.found }
	result.modules = {}
	done = false
	text = document()
end

local file = type(outputPath) == "string" and io.open(outputPath, "wb")
if not file then
	io.stderr:write("oracle.lua: cannot open the output file\n")
	os.exit(2)
end
file:write(text)
file:close()
os.exit(done and 0 or 1)
