-- loom oracle adapter: headless shims (Spike C, PR C2b).
-- Original code of this project (MIT); not part of Path of Building.
--
-- PoB v2.67.2's HeadlessWrapper.lua stubs `Inflate` (it returns ""), so
-- timeless jewel data (zlib streams in src/Data/TimelessJewelData/) would
-- load empty: a harness artifact, not PoB behavior. And PoB's jewel loader
-- caches the inflated data as a .bin file inside the checkout. oracle.lua
-- uses this module for two named shims, recorded in every output document:
--   inflate      replaces the global Inflate with zlib through LuaJIT FFI
--                (the pinned library: on Windows the checkout's own
--                runtime/zlib1.dll, loaded in place).
--   write_guard  replaces io.open and refuses every write mode; the
--                adapter keeps the original for its own output file.
--                It covers io.open only (not io.output, os.remove,
--                os.rename, io.popen, or FFI); the backstop is oracle-run's
--                clean check of the checkout, ignored files included,
--                after each fixture.
-- Inputs are PoB's own pinned data, so the inflated size is not capped.
-- Loaded with dofile by oracle.lua and by
-- tools/oracle/fixtures/lua-tests/shims.lua.

local ffi = require("ffi")

local M = {}

ffi.cdef([[
typedef struct loom_z_stream {
	const unsigned char *next_in;
	unsigned int avail_in;
	unsigned long total_in;
	unsigned char *next_out;
	unsigned int avail_out;
	unsigned long total_out;
	const char *msg;
	void *state;
	void *zalloc;
	void *zfree;
	void *opaque;
	int data_type;
	unsigned long adler;
	unsigned long reserved;
} loom_z_stream;
const char *zlibVersion(void);
int inflateInit_(loom_z_stream *strm, const char *version, int stream_size);
int inflate(loom_z_stream *strm, int flush);
int inflateEnd(loom_z_stream *strm);
]])

if ffi.os == "Windows" then
	ffi.cdef("void *LoadLibraryExA(const char *name, void *file, uint32_t flags);")
end

-- LoadLibraryExA flag: resolve the library's own dependencies (zlib1.dll
-- needs VCRUNTIME140.dll, which PoB ships next to it) from its folder.
local LOAD_WITH_ALTERED_SEARCH_PATH = 0x8

local Z_OK, Z_STREAM_END, Z_NO_FLUSH = 0, 1, 0
local CHUNK = 1048576

-- Loads zlib from `library` (an absolute path on Windows, a library name
-- such as libz.so.1 elsewhere). Returns { lib, version } or nil and a
-- message without the path (the message goes into the output document,
-- which must not depend on where the oracle lives).
function M.loadZlib(library)
	if type(library) ~= "string" or library == "" then
		return nil, "no zlib library given"
	end
	local ok, lib = pcall(function()
		if ffi.os == "Windows" then
			if ffi.C.LoadLibraryExA(library, nil, LOAD_WITH_ALTERED_SEARCH_PATH) == nil then
				error("LoadLibraryExA failed", 0)
			end
		end
		return ffi.load(library)
	end)
	if not ok then
		-- Without the path, and ASCII only (the system's message may be in
		-- another encoding), so the document stays valid and portable.
		local message = tostring(lib):gsub(library:gsub("%p", "%%%0"), "<zlib>"):gsub("[^\32-\126]", "?")
		return nil, "cannot load the zlib library: " .. message
	end
	local okVersion, version = pcall(function()
		return ffi.string(lib.zlibVersion())
	end)
	if not okVersion then
		return nil, "the zlib library has no zlibVersion"
	end
	return { lib = lib, version = version }
end

-- Inflates one zlib stream. Returns the data, the stream's Adler-32 (the
-- checksum of the inflated bytes, checked by zlib), and the number of
-- input bytes after the end of the stream; or nil and a message.
function M.inflate(zlib, data)
	if type(data) ~= "string" then
		return nil, "input is a " .. type(data) .. ", not a string"
	end
	local lib = zlib.lib
	local stream = ffi.new("loom_z_stream[1]")
	local rc = lib.inflateInit_(stream, lib.zlibVersion(), ffi.sizeof("loom_z_stream"))
	if rc ~= Z_OK then
		return nil, "inflateInit_ returned " .. rc
	end
	local s = stream[0]
	-- zlib reads from a copy owned by an FFI object that stays referenced
	-- until inflateEnd (a pointer cast from a Lua string holds no
	-- reference, and a compiled trace need not keep `data` alive).
	if #data >= 0xffffffff then
		lib.inflateEnd(stream)
		return nil, "input of " .. #data .. " bytes is too large for one zlib call"
	end
	local input = ffi.new("unsigned char[?]", #data + 1)
	ffi.copy(input, data, #data)
	s.next_in = input
	s.avail_in = #data
	local buffer = ffi.new("unsigned char[?]", CHUNK)
	local parts = {}
	repeat
		s.next_out = buffer
		s.avail_out = CHUNK
		rc = lib.inflate(stream, Z_NO_FLUSH)
		local produced = CHUNK - s.avail_out
		if produced > 0 then
			parts[#parts + 1] = ffi.string(buffer, produced)
		end
	until rc ~= Z_OK
	local adler = tonumber(s.adler)
	local trailing = tonumber(s.avail_in)
	local message = s.msg ~= nil and ffi.string(s.msg) or nil
	lib.inflateEnd(stream)
	input = nil -- referenced until here on purpose
	if rc ~= Z_STREAM_END then
		return nil, "inflate returned " .. rc .. (message and (" (" .. message .. ")") or "")
	end
	return table.concat(parts), adler, trailing
end

-- An io.open that refuses every mode that writes ("w", "a", or "+"),
-- records the refused path in `refused`, and fails the way io.open fails
-- (nil and a message), which PoB's loader ignores.
function M.writeGuard(open, refused)
	return function(path, mode)
		if type(mode) == "string" and mode:find("[wa+]") then
			refused[#refused + 1] = tostring(path)
			return nil, tostring(path) .. ": write refused by the loom oracle write guard"
		end
		return open(path, mode)
	end
end

return M
