-- Shared by the fake adapters (loom, Spike C). Each fake stands in for
-- tools/oracle/adapter/oracle.lua and misbehaves in one way, so the Rust
-- test `fake_adapters_should_get_their_class_on_the_real_runtime` can
-- check the class oracle-run assigns. Original code; nothing from PoB.
local M = {}

function M.args(...)
	local fixture, output = ...
	local names = {}
	for i = 3, select("#", ...) do
		local kind, value = select(i, ...):match("^(%a+)=(.+)$")
		if kind == "breakdown" then
			names[#names + 1] = value
		end
	end
	return fixture, output, names
end

local SOURCE = '"breakdown_source":"calcsTab.calcsEnv.player.breakdown"'
local SETTINGS = '"settings":{"path":"/Settings.xml","found":false}'
-- The shim records and an empty probe (protocol 4).
local SHIMS = '"shims":[{"name":"inflate","replaces":"Inflate","installed":true,"detail":"zlib (fake)"},'
	.. '{"name":"variant_order","replaces":"pairs","installed":true,"detail":"0 ambiguous gem variant tables"},'
	.. '{"name":"write_guard","replaces":"io.open","installed":true,"detail":null}],'
	.. '"probe":{"inflate":[],"variant_order":[],"writes_refused":[],"calcs_view":[]}'

-- A complete document; `mainOutput` is JSON text.
function M.done(names, mainOutput, protocol)
	local parts = {}
	for _, name in ipairs(names) do
		parts[#parts + 1] = '"' .. name .. '":null'
	end
	return '{"protocol":' .. (protocol or 4) .. ',"stage":"done","error":null,"partial":[],"modules":[],'
		.. SETTINGS .. "," .. SHIMS .. "," .. SOURCE .. ',"breakdowns":{' .. table.concat(parts, ",") .. '},"main_output":' .. mainOutput .. "}"
end

-- A failed stage; `message` must not need JSON escaping.
function M.failed(stage, source, message)
	local settings = stage == "startup" and '"settings":null' or SETTINGS
	return '{"protocol":4,"stage":"' .. stage .. '","error":{"source":"' .. source .. '","message":"' .. message
		.. '"},"partial":[],"modules":[],' .. settings .. "," .. SHIMS .. "," .. SOURCE .. ',"breakdowns":null,"main_output":null}'
end

function M.write(path, text)
	local file = assert(io.open(path, "wb"))
	file:write(text, "\n")
	file:close()
end

return M
