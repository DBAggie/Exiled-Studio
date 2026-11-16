-- loom PoB custom-line parse (ADR-015 Amendment 2; Phase 3 close-out).
-- Original code of this project (MIT); not part of Path of Building.
-- Maintainer-run, not in CI, and outside the oracle adapter and its
-- protocol (it reads no build).
--
-- Reads one custom modifier line per input line (custom_lines.py writes
-- them from PoB build files) and writes, as one JSON array, each line as
-- PoB v2.67.2's config tab reads it (`StripEscapes`, then trimmed:
-- Classes/ConfigTab.lua 1114) with PoB's `modLib.parseMod` result: the
-- mods (name, type, value, flags, keyword flags, tags) and the text left
-- over (`extra`, or false). PoB applies a line only when `mods and not
-- extra` (ConfigTab.lua 1116). `loom-parity-cli custom-lines` compares it
-- with our reading.
--   luajit pob_parse_lines.lua <lines.txt> <out.json>
-- run like pob_data.lua (README.md). Writes go only to <out.json>.

---@diagnostic disable: undefined-global

local inPath, outPath = ...
local dataDir = (arg[0]:gsub("[^/\\]+$", ""))
local adapterDir = dataDir .. "../adapter/"

local function die(message)
	io.stderr:write("pob_parse_lines.lua: ", message, "\n")
	os.exit(1)
end

if not inPath or not outPath then
	die("usage: luajit pob_parse_lines.lua <lines.txt> <out.json>")
end

local json = dofile(adapterDir .. "json.lua")
local shims = dofile(adapterDir .. "shims.lua")
local openFile = io.open
local input = {}
for line in io.lines(inPath) do
	input[#input + 1] = line
end
local refused = {}
io.open = shims.writeGuard(openFile, refused)

dofile("HeadlessWrapper.lua")
if launch.promptMsg then
	die("PoB startup: " .. tostring(launch.promptMsg))
end

-- Plain values of a table (one level), as the JSON writer takes them.
local function plain(t)
	local out = {}
	for k, v in pairs(t) do
		if type(v) ~= "table" and type(v) ~= "function" then
			out[k] = v
		end
	end
	return out
end

local out = {}
local seen = {}
for _, raw in ipairs(input) do
	local line = StripEscapes(raw):match("^%s*(.-)%s*$")
	if line ~= "" and not seen[line] then
		seen[line] = true
		local mods, extra = modLib.parseMod(line)
		local entry = { line = line, extra = extra or false, mods = false }
		if mods then
			local list = {}
			for i = 1, #mods do
				local m = mods[i]
				local tags = {}
				for j = 1, #m do
					tags[#tags + 1] = plain(m[j])
				end
				local value = m.value
				if type(value) == "table" then
					value = plain(value)
				end
				list[#list + 1] = {
					name = m.name, type = m.type, value = value,
					flags = m.flags, keywordFlags = m.keywordFlags, tags = tags,
				}
			end
			entry.mods = list
		end
		out[#out + 1] = entry
	end
end
table.sort(out, function(a, b) return a.line < b.line end)

local file = openFile(outPath, "wb")
if not file then
	die("cannot write " .. outPath)
end
file:write(json.encode(out, "lines"), "\n")
file:close()
if #refused > 0 then
	die("PoB tried to write: " .. table.concat(refused, ", "))
end
