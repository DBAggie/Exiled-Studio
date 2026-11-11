-- loom share-code maker (Phase 2 plan, step 10). Original code of this
-- project (MIT); not part of Path of Building. Maintainer-run, not in CI.
--
-- Makes a PoB share code with PoB v2.67.2's own code on the pinned oracle
-- runtime, for the share-code fixtures (tools/oracle/fixtures/share-codes/):
--   luajit share_code.lua <fixture.xml> <out.code> <out.xml> zlib=<library>
-- run like oracle.lua (working directory <oracle-dir>/pob/src, LUA_PATH and
-- LUA_CPATH as `oracle-run` sets them; see the README there).
--
-- It loads the fixture with the wrapper's loadBuildFromXML, then writes
-- what PoB's "Generate" button makes (Classes/ImportTab.lua, line 504):
--   common.base64.encode(Deflate(self.build:SaveDB("code"))):gsub("+","-"):gsub("/","_")
-- and, next to it, the XML that SaveDB returned (the code's source).
--
-- PoB's Deflate is a native function of its GUI host; HeadlessWrapper.lua
-- stubs it (returns ""). Here it is zlib's compress2 at the default level,
-- from the same zlib library oracle.lua inflates with (PoB's own
-- runtime/zlib1.dll on Windows). Any zlib stream reads back the same, so
-- the code's content is PoB's: its XML, its base64, its alphabet swap.
-- Inflate is the oracle's inflate shim (jewel data loads with a build).
-- No PoB file is changed; writes go only to the two output paths.

---@diagnostic disable: undefined-global

local fixturePath, codePath, xmlPath, zlibArgument = ...
local adapterDir = (arg[0]:gsub("[^/\\]+$", ""))
local zlibLibrary = zlibArgument and zlibArgument:match("^zlib=(.+)$")

local function die(message)
	io.stderr:write("share_code.lua: ", message, "\n")
	os.exit(1)
end

if not (fixturePath and codePath and xmlPath and zlibLibrary) then
	die("usage: luajit share_code.lua <fixture.xml> <out.code> <out.xml> zlib=<library>")
end

local ffi = require("ffi")
local shims = dofile(adapterDir .. "shims.lua")
local openFile = io.open
local refused = {}
io.open = shims.writeGuard(openFile, refused)

local zlib, zlibError = shims.loadZlib(zlibLibrary)
if not zlib then
	die(zlibError)
end

ffi.cdef([[
unsigned long compressBound(unsigned long sourceLen);
int compress2(unsigned char *dest, unsigned long *destLen, const unsigned char *source, unsigned long sourceLen, int level);
]])
local Z_OK, Z_DEFAULT_COMPRESSION = 0, -1

local function deflate(data)
	local bound = zlib.lib.compressBound(#data)
	local out = ffi.new("unsigned char[?]", bound)
	local outLen = ffi.new("unsigned long[1]", bound)
	local input = ffi.new("unsigned char[?]", #data + 1)
	ffi.copy(input, data, #data)
	local rc = zlib.lib.compress2(out, outLen, input, #data, Z_DEFAULT_COMPRESSION)
	if rc ~= Z_OK then
		error("compress2 returned " .. rc)
	end
	return ffi.string(out, outLen[0])
end

dofile("HeadlessWrapper.lua")
Inflate = function(data)
	local inflated, err = shims.inflate(zlib, data)
	if not inflated then
		die("inflate: " .. err)
	end
	return inflated
end
Deflate = deflate
if launch.promptMsg then
	die("PoB startup: " .. tostring(launch.promptMsg))
end

local file = openFile(fixturePath, "rb")
if not file then
	return die("cannot open the fixture")
end
local fixture = file:read("*a")
file:close()
loadBuildFromXML(fixture, "fixture")
if launch.promptMsg then
	die("PoB load: " .. tostring(launch.promptMsg))
end

-- PoB's expression, with the build PoB has loaded.
local xmlText = build:SaveDB("code")
if type(xmlText) ~= "string" then
	die("SaveDB returned no text")
end
local code = common.base64.encode(Deflate(xmlText)):gsub("+", "-"):gsub("/", "_")

local function write(path, text)
	local out = openFile(path, "wb")
	if not out then
		return die("cannot write " .. path)
	end
	out:write(text)
	out:close()
end
write(codePath, code)
write(xmlPath, xmlText)
-- A refused write is PoB caching data it loaded (the timeless jewel
-- .bin files, Spike C); the build in memory is the same either way.
for _, path in ipairs(refused) do
	print("refused write: " .. tostring(path))
end
print(("zlib %s; xml %d bytes; code %d characters"):format(zlib.version, #xmlText, #code))
