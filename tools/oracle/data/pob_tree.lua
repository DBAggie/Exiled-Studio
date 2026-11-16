-- loom PoB passive tree export (passive tree plan, T1, T7, T8). Original
-- code of this project (MIT); not part of Path of Building. Maintainer-run,
-- not in CI, and outside the oracle adapter and its protocol.
--
--   luajit pob_tree.lua layout <out.json> [tree=<version>] [zlib=<library>]
--   luajit pob_tree.lua alloc <requests.txt> <out.json> [tree=<version>] [zlib=<library>]
--   luajit pob_tree.lua readback <build.xml> <out.json> [zlib=<library>]
--
-- zlib (the manifest's library, as oracle-run passes it) installs the
-- oracle's inflate shim; readback needs it for a build with a timeless
-- jewel.
--
-- run like pob_data.lua (working directory <oracle-dir>/pob/src, LUA_PATH
-- and LUA_CPATH as tools/oracle/data/README.md shows).
--
-- layout: the tree as PoB's PassiveTree class builds it (positions from
--   PassiveTree.lua's ProcessNode, links from its `linkedId`), every node,
--   with each class's and ascendancy's start, and the nodes of PoB's
--   tree file that the class removed.
-- alloc: one request per line, `<classId> <ascendClassId> <target>
--   <allocated,...>` (the allocated list may be empty). For each, PoB's
--   spec is set with ImportFromNodeList on its latest tree (or the
--   `tree=` version: an older tree, passive tree plan T4/T8), then the
--   target's state is read: when not allocated, `path` (PoB's node.path,
--   from the target back to the allocated set) or none; when allocated,
--   `depends` (what DeallocNode would remove). Also the spec's allocated
--   set after the import (PoB prunes what does not connect) and
--   CountAllocNodes.
-- readback: PoB loads the build XML as its Load does, and each spec's
--   class, ascendancy, tree version, allocated nodes, and mastery effects
--   are written.
--
-- Writes go only to <out.json>: io.open is the oracle's write guard, and
-- a write it refuses is listed in the output as `writes_refused`.

---@diagnostic disable: undefined-global

local mode, input, output = ...
local dataDir = (arg[0]:gsub("[^/\\]+$", ""))
local adapterDir = dataDir .. "../adapter/"

local function die(message)
	io.stderr:write("pob_tree.lua: ", message, "\n")
	os.exit(1)
end

local treeArgument, zlibLibrary
local first = 4
if mode == "layout" then
	output = input
	input = nil
	first = 3
end
for i = first, select("#", ...) do
	local argument = select(i, ...)
	local kind, value = argument:match("^(%a+)=(.+)$")
	if kind == "tree" and (mode == "layout" or mode == "alloc") then
		treeArgument = value
	elseif kind == "zlib" then
		zlibLibrary = value
	else
		die("unknown argument " .. tostring(argument))
	end
end
if mode ~= "layout" and mode ~= "alloc" and mode ~= "readback" then
	die("usage: luajit pob_tree.lua layout <out.json> [tree=<version>] | alloc <requests.txt> <out.json> [tree=<version>] | readback <build.xml> <out.json>")
end
if not output then
	die("no output path")
end

local json = dofile(adapterDir .. "json.lua")
local shims = dofile(adapterDir .. "shims.lua")
local openFile = io.open
local refused = {}

local function readAll(path)
	local file, err = openFile(path, "rb")
	if not file then
		die("cannot read " .. path .. ": " .. tostring(err))
	end
	local text = file:read("*a")
	file:close()
	return text
end
local inputText = input and readAll(input)

io.open = shims.writeGuard(openFile, refused)
dofile("HeadlessWrapper.lua")
-- The oracle's inflate shim (adapter/shims.lua), when zlib is given: a
-- build with a timeless jewel needs PoB's compressed tables. The wrapper
-- defines its Inflate stub before PoB starts; jewel data loads only with
-- a build, so replacing it now is in time.
if zlibLibrary then
	local zlib, zlibError = shims.loadZlib(zlibLibrary)
	if not zlib then
		die(zlibError)
	end
	Inflate = function(data)
		local inflated, err = shims.inflate(zlib, data)
		if not inflated then
			die("inflate: " .. tostring(err))
		end
		return inflated
	end
end
if launch.promptMsg then
	die("PoB startup: " .. tostring(launch.promptMsg))
end

local function sortedKeys(map)
	local out = {}
	for k in pairs(map) do
		out[#out + 1] = k
	end
	table.sort(out)
	return out
end

local function ids(list)
	local seen, out = {}, {}
	for _, node in ipairs(list or {}) do
		local id = type(node) == "table" and node.id or node
		if not seen[id] then
			seen[id] = true
			out[#out + 1] = id
		end
	end
	table.sort(out)
	return out
end

local document
if mode == "layout" then
	local version = treeArgument or latestTreeVersion
	local tree = launch.main:LoadTree(version)
	local nodes = {}
	for id, node in pairs(tree.nodes) do
		local entry = {
			type = node.type,
			ascendancy = node.ascendancyName,
			group = node.g,
			orbit = node.o,
			orbit_index = node.oidx,
			links = ids(node.linkedId),
		}
		if node.x then
			entry.x = node.x
			entry.y = node.y
		end
		nodes[tostring(id)] = entry
	end
	-- Nodes of PoB's own tree file that its PassiveTree class removed
	-- (the legacy alternate ascendancies), with their ascendancy.
	local removed = {}
	local rawChunk = loadfile("TreeData/" .. version .. "/tree.lua")
	if not rawChunk then
		die("cannot read TreeData/" .. version .. "/tree.lua")
	end
	for key, node in pairs(rawChunk().nodes or {}) do
		local id = tonumber(key)
		if id and type(node) == "table" and node.skill and not tree.nodes[id] then
			removed[tostring(id)] = node.ascendancyName or ""
		end
	end
	local classes = {}
	for classId, class in pairs(tree.classes) do
		local ascendancies = {}
		for ascendId, ascend in pairs(class.classes or {}) do
			ascendancies[tostring(ascendId)] = { name = ascend.name, start = ascend.startNodeId }
		end
		classes[tostring(classId)] = { name = class.name, start = class.startNodeId, ascendancies = ascendancies }
	end
	document = {
		mode = mode,
		pob = { version = launch.versionNumber },
		tree_version = version,
		orbit_radii = tree.orbitRadii,
		nodes = nodes,
		removed = removed,
		classes = classes,
	}
elseif mode == "alloc" then
	newBuild()
	local spec = build.spec
	local allocVersion = treeArgument or latestTreeVersion
	local results = {}
	local line = 0
	for text in inputText:gmatch("[^\r\n]+") do
		line = line + 1
		local class, ascend, target, list = text:match("^(%d+) (%d+) (%d+) ?([%d,]*)$")
		if not class then
			die("requests line " .. line .. ": " .. text)
		end
		local hashes = {}
		for id in list:gmatch("%d+") do
			hashes[#hashes + 1] = tonumber(id)
		end
		-- Each request on a clean spec: PoB's ResetNodes keeps every
		-- ascendancy start allocated, so a bloodline's start from an earlier
		-- request would carry over (SelectSecondaryAscendClass(0) does not
		-- remove it).
		for _, node in pairs(spec.allocNodes) do
			node.alloc = false
		end
		wipeTable(spec.allocNodes)
		spec:ImportFromNodeList(nil, tonumber(class), tonumber(ascend), 0, hashes, {}, {}, allocVersion)
		local used, ascUsed = spec:CountAllocNodes()
		local node = spec.nodes[tonumber(target)]
		local result = {
			allocated = sortedKeys(spec.allocNodes),
			points = used,
			ascendancy_points = ascUsed,
		}
		if not node then
			result.missing = true
		elseif node.alloc then
			result.depends = ids(node.depends)
		elseif node.path then
			local path = {}
			for i, n in ipairs(node.path) do
				path[i] = n.id
			end
			result.path = path
		end
		results[line] = result
	end
	document = { mode = mode, pob = { version = launch.versionNumber }, tree_version = allocVersion, results = results }
else
	local loaded, loadError = xpcall(function()
		loadBuildFromXML(inputText, "readback")
	end, debug.traceback)
	if not loaded then
		die("loading the build: " .. tostring(loadError))
	end
	-- Errors inside PoB's frame are caught by launch:OnFrame and shown as a
	-- prompt.
	if launch.promptMsg then
		die("PoB: " .. tostring(launch.promptMsg))
	end
	local specs = {}
	for i, spec in ipairs(build.treeTab.specList) do
		local masteries = {}
		for node, effect in pairs(spec.masterySelections) do
			masteries[#masteries + 1] = { node = node, effect = effect }
		end
		table.sort(masteries, function(a, b) return a.node < b.node end)
		local jewels = {}
		for node, item in pairs(spec.jewels) do
			if item and item ~= 0 then
				jewels[#jewels + 1] = { node = node, item = item }
			end
		end
		table.sort(jewels, function(a, b) return a.node < b.node end)
		specs[i] = {
			title = spec.title,
			tree_version = spec.treeVersion,
			class = spec.curClassId,
			ascendancy = spec.curAscendClassId,
			nodes = sortedKeys(spec.allocNodes),
			masteries = masteries,
			jewels = jewels,
		}
	end
	document = {
		mode = mode,
		pob = { version = launch.versionNumber },
		class = build.spec.curClassName,
		ascendancy = build.spec.curAscendClassName,
		active = build.treeTab.activeSpec,
		specs = specs,
	}
end

-- A refused write (PoB caching its timeless jewel tables, for example)
-- did not happen; it is recorded, and allocations do not read it.
if #refused > 0 then
	document.writes_refused = refused
end
local ok, text = pcall(json.encode, document, "document")
if not ok then
	if type(text) == "table" then
		die("JSON: " .. tostring(text.path) .. ": " .. tostring(text.message))
	end
	die("JSON: " .. tostring(text))
end
local file, err = openFile(output, "wb")
if not file then
	die("cannot write the output: " .. tostring(err))
end
file:write(text, "\n")
file:close()
