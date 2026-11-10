-- Tests of the adapter's JSON writer (tools/oracle/adapter/json.lua) on
-- the pinned runtime (loom, Spike C; original code). Run by the Rust test
-- `json_writer_should_pass_its_lua_tests` (ignored without an acquired
-- oracle):
--   luajit json_writer.lua <path of json.lua>
-- Prints one line per failed case and exits 1, or prints "ok <n>" and
-- exits 0.

local json = dofile(...)

local failures, cases = {}, 0

local function expect(name, value, expected, opaque)
	cases = cases + 1
	local ok, text = pcall(json.encode, value, "v", opaque)
	if not ok then
		failures[#failures + 1] = name .. ": raised " .. tostring(type(text) == "table" and text.message or text)
	elseif text ~= expected then
		failures[#failures + 1] = name .. ": got " .. text .. " expected " .. expected
	end
end

local function expectError(name, value, pattern)
	cases = cases + 1
	local ok, err = pcall(json.encode, value, "v")
	if ok then
		failures[#failures + 1] = name .. ": no error, wrote " .. err
	elseif getmetatable(err) ~= json.NormalizationError then
		failures[#failures + 1] = name .. ": not a normalization error: " .. tostring(err)
	elseif not err.message:find(pattern, 1, true) then
		failures[#failures + 1] = name .. ": message " .. err.message
	end
end

local Q, B = string.char(34), string.char(92) -- quote, backslash

-- Strings
expect("plain", "abc", Q .. "abc" .. Q)
expect("quote and backslash", "a" .. Q .. B .. "b", Q .. "a" .. B .. Q .. B .. B .. "b" .. Q)
expect("short escapes", "\n\r\t\b\f", Q .. B .. "n" .. B .. "r" .. B .. "t" .. B .. "b" .. B .. "f" .. Q)
expect("other control and DEL", string.char(1, 31, 127), Q .. B .. "u0001" .. B .. "u001f" .. B .. "u007f" .. Q)
expect("UTF-8 kept", "h\195\169llo \226\130\172 \240\159\152\128", Q .. "h\195\169llo \226\130\172 \240\159\152\128" .. Q)
expectError("invalid UTF-8 byte", "a\255", "not valid UTF-8")
expectError("truncated sequence", "a\195", "not valid UTF-8")
expectError("overlong", "\192\175", "not valid UTF-8")
expectError("surrogate", "\237\160\128", "not valid UTF-8")
expectError("above U+10FFFF", "\244\144\128\128", "not valid UTF-8")

-- Numbers
expect("integer", 6728, "6728")
expect("fraction", 0.1, "0.10000000000000001")
expect("negative zero", -0.0, "-0")
expect("large", 1e300, "1.0000000000000001e+300")
expect("nan", 0 / 0, '{"$number":"nan"}')
expect("inf", math.huge, '{"$number":"inf"}')
expect("-inf", -math.huge, '{"$number":"-inf"}')
expect("booleans", { true, false }, "[true,false]")

-- Tables
expect("empty table", {}, "{}")
expect("sequence", { 1, "a" }, '[1,"a"]')
expect("sequence with hole", { 1, nil, 3 }, '{"#1":1,"#3":3}')
expect("index and name", { 1, x = 2 }, '{"#1":1,"x":2}')
expect("sorted keys", { b = 1, a = 2, B = 3 }, '{"B":3,"a":2,"b":1}')
expect("number keys", { [0] = "z", [2.5] = "f", [-1] = "n" }, '{"#-1":"n","#0":"z","#2.5":"f"}')
expect("infinite key", { [math.huge] = 1 }, '{"#inf":1}')
expect("nested", { a = { b = { 1 } } }, '{"a":{"b":[1]}}')
local cyclic = { name = "root" }
cyclic.self = cyclic
expect("cycle", cyclic, '{"name":"root","self":{"$cycle":"v"}}')
local shared = { 1 }
expect("shared table twice", { a = shared, b = shared }, '{"a":[1],"b":[1]}')
expect("function", { f = print }, '{"f":{"$function":true}}')
expect("class instance", { item = setmetatable({}, { _className = "Item" }) }, '{"item":{"$object":"Item"}}')
expect("plain metatable", { t = setmetatable({}, {}) }, '{"t":{"$object":"table"}}')
expect("opaque key", { sourceGem = { big = 1 }, other = { 1 } }, '{"other":[1],"sourceGem":{"$opaque":"sourceGem"}}', { sourceGem = true })
expect("opaque key only for tables", { sourceGem = 5 }, '{"sourceGem":5}', { sourceGem = true })
expect("userdata", { u = io.stdout }, '{"u":{"$userdata":"userdata"}}')
expectError("dollar key", { ["$x"] = 1 }, "collides")
expectError("hash key", { ["#1"] = 1 }, "collides")
expectError("table key", { [{}] = 1 }, "key of type table")
expectError("boolean key", { [true] = 1 }, "key of type boolean")
local deep = {}
local node = deep
for _ = 1, json.MAX_DEPTH + 5 do
	node.next = {}
	node = node.next
end
expectError("too deep", deep, "nested deeper")

if #failures > 0 then
	for _, line in ipairs(failures) do
		print(line)
	end
	os.exit(1)
end
print("ok " .. cases)
os.exit(0)
