-- Tests of the adapter's headless shims (tools/oracle/adapter/shims.lua)
-- on the pinned runtime (loom, Spike C, PR C2b; original code). Run by
-- the Rust test `shims_should_pass_their_lua_tests` (ignored without an
-- acquired oracle):
--   luajit shims.lua <path of shims.lua> <zlib library>
-- The zlib library is what oracle-run passes as zlib= (an absolute path on
-- Windows, a library name on Linux). Prints one line per failed case and
-- exits 1, or prints "ok <n>" and exits 0.

local shimsPath, library = ...
local shims = dofile(shimsPath)

local failures, cases = {}, 0

local function check(name, condition, detail)
	cases = cases + 1
	if not condition then
		failures[#failures + 1] = name .. (detail and (": " .. tostring(detail)) or "")
	end
end

local function fromHex(hex)
	return (hex:gsub("%x%x", function(byte)
		return string.char(tonumber(byte, 16))
	end))
end

-- zlib.compress(b"loom oracle " * 500, 9) in Python: 6000 bytes, Adler-32
-- 3057624413, 49 bytes compressed.
local STREAM = fromHex("78daedc6b10900200c04c055b29a885de421fb17ee607d575d27b7326bf7a97677777777777777777777f7af3fb63fa55d")
local PLAIN = string.rep("loom oracle ", 500)

-- Loading
local zlib, err = shims.loadZlib(library)
check("load zlib", zlib, err)
if not zlib then
	print(table.concat(failures, "\n"))
	os.exit(1)
end
check("zlib version", zlib.version:match("^1%.%d"), zlib.version)

local none, noneError = shims.loadZlib(nil)
check("no library given", none == nil and noneError == "no zlib library given", noneError)

local missing = "/loom-no-such-dir/zlib-missing-library.dll"
local bad, badError = shims.loadZlib(missing)
check("missing library fails", bad == nil, badError)
check("missing library message has no path", type(badError) == "string" and not badError:find("loom-no-such-dir", 1, true), badError)

-- Inflate
local out, adler, trailing = shims.inflate(zlib, STREAM)
check("inflate data", out == PLAIN, out and #out)
check("inflate adler32", adler == 3057624413, adler)
check("inflate no trailing bytes", trailing == 0, trailing)

out, adler, trailing = shims.inflate(zlib, STREAM .. "extra")
check("trailing bytes counted", out == PLAIN and trailing == 5, trailing)

out = shims.inflate(zlib, STREAM)
check("inflate is repeatable", out == PLAIN)

out, err = shims.inflate(zlib, STREAM:sub(1, 20))
check("truncated stream fails", out == nil and err:find("inflate returned", 1, true), err)

local corrupt = STREAM:sub(1, 10) .. string.char((STREAM:byte(11) + 1) % 256) .. STREAM:sub(12)
out, err = shims.inflate(zlib, corrupt)
check("corrupt stream fails", out == nil and type(err) == "string", err)

out, err = shims.inflate(zlib, "not zlib at all")
check("wrong header fails", out == nil and err:find("incorrect header check", 1, true), err)

out, err = shims.inflate(zlib, nil)
check("non-string input fails", out == nil and err == "input is a nil, not a string", err)

out, err = shims.inflate(zlib, "")
check("empty input fails", out == nil and type(err) == "string", err)

-- More than one output chunk (the shim inflates in 1 MiB steps).
local ffi = require("ffi")
ffi.cdef("int compress2(unsigned char *dest, unsigned long *destLen, const unsigned char *source, unsigned long sourceLen, int level);")
local large = string.rep("0123456789abcdef", 200000) -- 3.2 MB
local destLen = ffi.new("unsigned long[1]", #large + 1024)
local dest = ffi.new("unsigned char[?]", #large + 1024)
check("compress2", zlib.lib.compress2(dest, destLen, large, #large, 6) == 0)
out = shims.inflate(zlib, ffi.string(dest, destLen[0]))
check("multi-chunk inflate", out == large, out and #out)

-- Write guard
local opened = {}
local function fakeOpen(path, mode)
	opened[#opened + 1] = path .. "|" .. tostring(mode)
	return "handle"
end
local refused = {}
local guarded = shims.writeGuard(fakeOpen, refused)
for _, mode in ipairs({ "w", "wb", "wb+", "a", "ab", "r+", "rb+", "a+" }) do
	local handle, message = guarded("./Data/x.bin", mode)
	check("refuses " .. mode, handle == nil and message:find("write refused", 1, true), message)
end
check("refused paths recorded", #refused == 8 and refused[1] == "./Data/x.bin", #refused)
check("refused opens never reach io.open", #opened == 0, #opened)
for _, mode in ipairs({ "r", "rb" }) do
	check("passes " .. mode, guarded("./Data/x.zip", mode) == "handle")
end
check("passes the default mode", guarded("./Data/x.zip") == "handle")
check("passes the arguments unchanged", opened[3] == "./Data/x.zip|nil", opened[3])
check("refused list unchanged by reads", #refused == 8)

-- variant_order: registration
local base = { name = "base" }
local gemsByGameId = {
	-- A Vaal gem with transfigured copies: three variants, one grantedEffectId.
	vaal = {
		VaalX = { grantedEffectId = "VaalX" },
		VaalXAltX = { grantedEffectId = "VaalX" },
		VaalXAltY = { grantedEffectId = "VaalX" },
	},
	-- Transfigured variants with their own grantedEffectIds: not ambiguous.
	plain = { X = { grantedEffectId = "X" }, XAltX = { grantedEffectId = "XAltX" } },
	single = { Y = { grantedEffectId = "Y" } },
	broken = "not a table",
}
local ordered = {}
local count = shims.registerAmbiguousVariants(gemsByGameId, ordered, pairs)
check("registers only ambiguous tables", count == 1 and ordered[gemsByGameId.vaal] == "vaal", count)
check("leaves distinct variants alone", ordered[gemsByGameId.plain] == nil and ordered[gemsByGameId.single] == nil)

-- variant_order: iteration
local seen, unordered = {}, {}
local orderedPairs = shims.orderedPairs(pairs, ordered, function(label)
	seen[#seen + 1] = label
end, function(label)
	unordered[#unordered + 1] = label
end)
local keys = {}
for k, v in orderedPairs(gemsByGameId.vaal) do
	keys[#keys + 1] = k
	check("value of " .. k, v == gemsByGameId.vaal[k])
end
check("sorted order, base first", table.concat(keys, ",") == "VaalX,VaalXAltX,VaalXAltY", table.concat(keys, ","))
check("ordered use reported", #seen == 1 and seen[1] == "vaal", #seen)
-- The first match wins, as in PoB's SkillsTab:Load: always the base gem.
local picked
for _, variant in orderedPairs(gemsByGameId.vaal) do
	if variant.grantedEffectId == "VaalX" then
		picked = variant
		break
	end
end
check("first match is the base gem", picked == gemsByGameId.vaal.VaalX)
-- Other tables: exactly the original pairs (same iterator function).
local f, t, k0 = orderedPairs(gemsByGameId.plain)
check("other tables use next", f == next and t == gemsByGameId.plain and k0 == nil)
check("other tables not reported", #seen == 2, #seen)
local count2 = 0
for _ in orderedPairs({ a = 1, b = 2, [base] = 3 }) do
	count2 = count2 + 1
end
check("unregistered table with a table key", count2 == 3, count2)
-- A registered table with a key that is not a string falls back.
local mixed = { a = 1, [1] = 2 }
ordered[mixed] = "mixed"
local mixedCount = 0
for _ in orderedPairs(mixed) do
	mixedCount = mixedCount + 1
end
check("non-string key falls back to pairs", mixedCount == 2 and #seen == 2, mixedCount)
check("fallback reported", #unordered == 1 and unordered[1] == "mixed", #unordered)
-- An empty registered table iterates nothing.
local empty = {}
ordered[empty] = "empty"
local emptyCount = 0
for _ in orderedPairs(empty) do
	emptyCount = emptyCount + 1
end
check("empty registered table", emptyCount == 0)

if #failures > 0 then
	print(table.concat(failures, "\n"))
	os.exit(1)
end
print("ok " .. cases)
os.exit(0)
