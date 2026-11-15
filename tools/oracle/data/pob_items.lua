-- loom PoB item copy export (B-2 item evidence). Original code of this
-- project (MIT); not part of Path of Building. Maintainer-run, not in CI,
-- and outside the oracle adapter and its protocol (it reads no build).
--
-- For each game item copy given, PoB v2.67.2 reads it as its item paste
-- does (`new("Item", raw)`: `ItemClass:ParseRaw(sanitiseText(raw))`) and
-- writes the text it keeps (`ItemClass:BuildRaw`) to <out-dir>/<name>.txt:
--   luajit pob_items.lua <out-dir> <copy.txt>...
-- run like pob_data.lua (working directory <oracle-dir>/pob/src; see
-- tools/oracle/data/README.md). The paste's later `NormaliseQuality` (which
-- raises quality to 20 on items that are not unique or corrupted) is left
-- out: the comparison is of the parse, and the copy's quality is the
-- game's. Writes go only to <out-dir>.

---@diagnostic disable: undefined-global

local outDir = ...
local inputs = { select(2, ...) }
local dataDir = (arg[0]:gsub("[^/\\]+$", ""))
local adapterDir = dataDir .. "../adapter/"

local function die(message)
	io.stderr:write("pob_items.lua: ", message, "\n")
	os.exit(1)
end

if not outDir or #inputs == 0 then
	die("usage: luajit pob_items.lua <out-dir> <copy.txt>...")
end

-- Read every copy before the write guard is installed.
local copies = { }
for _, path in ipairs(inputs) do
	local file = io.open(path, "rb")
	if not file then
		die("cannot read " .. path)
	end
	local name = path:match("([^/\\]+)%.txt$") or die("not a .txt file: " .. path)
	copies[#copies + 1] = { name = name, raw = file:read("*a") }
	file:close()
end

local shims = dofile(adapterDir .. "shims.lua")
local openFile = io.open
local refused = { }
io.open = shims.writeGuard(openFile, refused)

dofile("HeadlessWrapper.lua")
if launch.promptMsg then
	die("PoB startup: " .. tostring(launch.promptMsg))
end

for _, copy in ipairs(copies) do
	local item = new("Item", copy.raw)
	local text = item.base and item:BuildRaw() or ("no base: " .. tostring(item.name))
	local out = openFile(outDir .. "/" .. copy.name .. ".txt", "wb")
	if not out then
		die("cannot write " .. copy.name)
	end
	out:write((text:gsub("\r\n", "\n")), "\n")
	out:close()
end
if #refused > 0 then
	die("refused writes: " .. table.concat(refused, ", "))
end
