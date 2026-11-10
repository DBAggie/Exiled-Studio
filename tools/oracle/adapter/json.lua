-- loom oracle adapter: the JSON writer (protocol 1). Original code of this
-- project (MIT); not part of Path of Building. Loaded by oracle.lua with
-- dofile; tested on the real runtime by
-- tools/oracle/fixtures/lua-tests/json_writer.lua.
--
-- Lua tables become JSON arrays when their keys are exactly 1..n (n >= 1),
-- otherwise objects with sorted keys (an empty table is {}); a number key
-- k is written as "#" .. %.17g (#inf, #-inf for infinities). Values that
-- are not data become one-key placeholder objects, never dropped:
--   {"$number": "inf" | "-inf" | "nan"}
--   {"$object": <PoB class name>}   a class instance (table with metatable)
--   {"$opaque": <key>}  a table under a key listed in `opaqueKeys` (input
--                       objects such as gem instances, whose game data is
--                       not calculation output)
--   {"$cycle": <path of the enclosing table>}
--   {"$function": true}, {"$userdata": <type>}
-- Normalization errors (raised as a NormalizationError value with `path`
-- and `message`): a string key starting with "$" or "#" (it could not be
-- told apart from the forms above), a key that is neither a string nor a
-- number, two keys written alike, a string that is not valid UTF-8, and
-- tables nested deeper than MAX_DEPTH (the CLI's JSON reader stops at 128
-- levels).

local M = {}

M.MAX_DEPTH = 100

local NormalizationError = {}
M.NormalizationError = NormalizationError

local function normalizationError(path, message)
	error(setmetatable({ path = path, message = message }, NormalizationError), 0)
end

local function formatNumber(n)
	if n ~= n then
		return '{"$number":"nan"}'
	elseif n == math.huge then
		return '{"$number":"inf"}'
	elseif n == -math.huge then
		return '{"$number":"-inf"}'
	end
	return string.format("%.17g", n)
end

local escapes = { ['"'] = '\\"', ["\\"] = "\\\\", ["\b"] = "\\b", ["\f"] = "\\f", ["\n"] = "\\n", ["\r"] = "\\r", ["\t"] = "\\t" }

-- True when `s` is valid UTF-8 (no overlongs, surrogates, or values above
-- U+10FFFF).
local function isUtf8(s)
	local i, n = 1, #s
	while i <= n do
		local c = s:byte(i)
		local len, min
		if c < 0x80 then
			len = 1
		elseif c >= 0xC2 and c <= 0xDF then
			len, min = 2, 0x80
		elseif c >= 0xE0 and c <= 0xEF then
			len, min = 3, 0x800
		elseif c >= 0xF0 and c <= 0xF4 then
			len, min = 4, 0x10000
		else
			return false
		end
		if len > 1 then
			if i + len - 1 > n then
				return false
			end
			local cp = c % (2 ^ (7 - len))
			for j = i + 1, i + len - 1 do
				local b = s:byte(j)
				if b < 0x80 or b > 0xBF then
					return false
				end
				cp = cp * 64 + (b - 0x80)
			end
			if cp < min or cp > 0x10FFFF or (cp >= 0xD800 and cp <= 0xDFFF) then
				return false
			end
		end
		i = i + len
	end
	return true
end

-- A JSON string; control characters (and DEL) as escapes.
function M.string(s, path)
	if s:find("[\128-\255]") and not isUtf8(s) then
		normalizationError(path, "string is not valid UTF-8")
	end
	local escaped = s:gsub('[%c"\\]', function(c)
		return escapes[c] or string.format("\\u%04x", c:byte())
	end)
	return '"' .. escaped .. '"'
end

local formatString = M.string

local function className(t)
	local mt = getmetatable(t)
	if type(mt) == "table" and type(mt._className) == "string" then
		return mt._className
	end
	return "table"
end

local function keyText(k, path)
	if type(k) == "number" then
		-- A NaN key cannot exist; infinities get a fixed spelling (the C
		-- library's "%g" spells them differently per platform).
		if k == math.huge then
			return "#inf"
		elseif k == -math.huge then
			return "#-inf"
		end
		return "#" .. string.format("%.17g", k)
	elseif type(k) == "string" then
		local first = k:sub(1, 1)
		if first == "$" or first == "#" then
			normalizationError(path, "key " .. k .. " collides with a placeholder or number key")
		end
		return k
	end
	normalizationError(path, "key of type " .. type(k))
end

local encodeValue

local function encodeTable(t, path, state, depth, out)
	if getmetatable(t) ~= nil then
		out[#out + 1] = '{"$object":' .. formatString(className(t), path) .. "}"
		return
	end
	if state.open[t] then
		out[#out + 1] = '{"$cycle":' .. formatString(state.open[t], path) .. "}"
		return
	end
	if depth > M.MAX_DEPTH then
		normalizationError(path, "tables nested deeper than " .. M.MAX_DEPTH)
	end
	state.open[t] = path
	local count, maxIndex, onlyIndices = 0, 0, true
	for k in pairs(t) do
		count = count + 1
		if type(k) == "number" and k >= 1 and k % 1 == 0 then
			if k > maxIndex then
				maxIndex = k
			end
		else
			onlyIndices = false
		end
	end
	if count > 0 and onlyIndices and maxIndex == count then
		out[#out + 1] = "["
		for i = 1, count do
			if i > 1 then
				out[#out + 1] = ","
			end
			encodeValue(t[i], path .. "[" .. i .. "]", state, depth + 1, out)
		end
		out[#out + 1] = "]"
	else
		local keys = {}
		for k in pairs(t) do
			keys[#keys + 1] = { text = keyText(k, path), key = k }
		end
		table.sort(keys, function(a, b)
			return a.text < b.text
		end)
		out[#out + 1] = "{"
		for i, entry in ipairs(keys) do
			if i > 1 then
				if keys[i - 1].text == entry.text then
					normalizationError(path, "two keys are written as " .. entry.text)
				end
				out[#out + 1] = ","
			end
			out[#out + 1] = formatString(entry.text, path)
			out[#out + 1] = ":"
			local value = t[entry.key]
			if type(value) == "table" and state.opaqueKeys[entry.key] then
				out[#out + 1] = '{"$opaque":' .. formatString(entry.text, path) .. "}"
			else
				encodeValue(value, path .. "." .. entry.text, state, depth + 1, out)
			end
		end
		out[#out + 1] = "}"
	end
	state.open[t] = nil
end

encodeValue = function(v, path, state, depth, out)
	local ty = type(v)
	if ty == "table" then
		encodeTable(v, path, state, depth, out)
	elseif ty == "number" then
		out[#out + 1] = formatNumber(v)
	elseif ty == "string" then
		out[#out + 1] = formatString(v, path)
	elseif ty == "boolean" then
		out[#out + 1] = v and "true" or "false"
	elseif ty == "function" then
		out[#out + 1] = '{"$function":true}'
	elseif ty == "nil" then
		out[#out + 1] = "null"
	else
		out[#out + 1] = '{"$userdata":' .. formatString(ty, path) .. "}"
	end
end

-- `v` as JSON text; `path` names it in errors; `opaqueKeys` is a set of
-- key names written as {"$opaque": key}.
function M.encode(v, path, opaqueKeys)
	local out = {}
	encodeValue(v, path, { open = {}, opaqueKeys = opaqueKeys or {} }, 1, out)
	return table.concat(out)
end

return M
