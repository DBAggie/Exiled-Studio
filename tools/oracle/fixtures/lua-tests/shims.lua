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

if #failures > 0 then
	print(table.concat(failures, "\n"))
	os.exit(1)
end
print("ok " .. cases)
os.exit(0)
