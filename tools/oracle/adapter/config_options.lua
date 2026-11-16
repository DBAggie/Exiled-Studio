-- Writes PoB's configuration options (Modules/ConfigOptions.lua) as JSON
-- for the configuration page's catalogue (UX sprint 2): each option's key,
-- type, label, section, the heading it sits under, list choices, default,
-- static tooltip, the skills it is shown for, and whether PoB shows it
-- only under other conditions. Labels and tooltips are PoB's (MIT), with
-- PoB's colour codes removed; tooltips PoB computes per build are left out.
--
-- Read only: loads PoB headless at the pinned oracle commit, writes nothing
-- but stdout (io.open refuses writes). Run in local/oracle/pob/src with the
-- oracle runtime's LUA_PATH and LUA_CPATH:
--   ../../bin/luajit <this file> <tag> <commit> > ui/src/data/pob-config-options.json
-- with the tag and commit of tools/oracle/manifest.json, after
-- `oracle-verify` (tools/oracle/config-options.ps1 does all this).

-- PoB's startup replaces the global `arg`: read it first.
local adapterDir = (arg[0]:gsub("[^/\\]+$", ""))
local tag, commit = arg[1], arg[2]
local json = dofile(adapterDir .. "json.lua")

local openFile = io.open
io.open = function(path, mode)
	if mode and mode:find("[wa+]") then
		return nil, "write refused: " .. tostring(path)
	end
	return openFile(path, mode)
end

-- PoB's startup prints progress; keep stdout for the JSON.
local realPrint = print
local out = io.stdout
print = function(...)
	io.stderr:write(table.concat({ ... }, "\t"), "\n")
end
ConPrintf = function(...)
	io.stderr:write(string.format(...), "\n")
end

dofile("HeadlessWrapper.lua")
print = realPrint

local list = LoadModule("Modules/ConfigOptions")

local function clean(s)
	s = s:gsub("%^x%x%x%x%x%x%x", ""):gsub("%^%d", "")
	return s
end

local function label(s)
	return (clean(s):gsub("%s*:%s*$", ""))
end

-- Visibility keys PoB checks against the build (ConfigVisibility.lua).
local CONDITIONS = {
	"ifCond", "ifCondTrue", "ifEnemyCond", "ifEnemyMult", "ifFlag", "ifMinionCond",
	"ifMod", "ifMult", "ifOption", "ifSkillData", "ifSkillFlag", "ifStat", "ifTagType",
}

local function names(v)
	if type(v) == "string" then
		return { v }
	elseif type(v) == "table" then
		local t = {}
		for _, x in ipairs(v) do
			t[#t + 1] = x
		end
		return t
	end
end

local options = {}
local section, heading, headingSkills = nil, nil, nil
for _, e in ipairs(list) do
	if e.section then
		section, heading, headingSkills = e.section, nil, nil
	elseif e.var == nil and type(e.label) == "string" then
		heading = label(e.label)
		headingSkills = names(e.ifSkill)
	elseif e.var then
		local o = {
			var = e.var,
			type = e.type,
			label = type(e.label) == "string" and label(e.label) or e.var,
			section = section,
			heading = heading,
		}
		if type(e.list) == "table" then
			o.list = {}
			for _, c in ipairs(e.list) do
				o.list[#o.list + 1] = { val = c.val, label = clean(tostring(c.label)) }
			end
			if e.defaultIndex and e.list[e.defaultIndex] then
				o.default = e.list[e.defaultIndex].val
			elseif e.list[1] then
				o.default = e.list[1].val
			end
		elseif e.defaultState ~= nil then
			o.default = e.defaultState
		end
		if e.defaultPlaceholderState ~= nil then
			o.placeholder = e.defaultPlaceholderState
		end
		if type(e.tooltip) == "string" then
			o.tooltip = clean(e.tooltip)
		end
		o.skills = names(e.ifSkill) or headingSkills
		local conditional = false
		for _, k in ipairs(CONDITIONS) do
			if e[k] ~= nil then
				conditional = true
			end
		end
		o.conditional = conditional
		if e.legacy then
			o.legacy = true
		end
		options[#options + 1] = o
	end
end

out:write(json.encode({
	source = {
		repository = "https://github.com/PathOfBuildingCommunity/PathOfBuilding",
		tag = tag,
		commit = commit,
		file = "src/Modules/ConfigOptions.lua",
		license = "MIT",
	},
	options = options,
}, "options"))
out:write("\n")
