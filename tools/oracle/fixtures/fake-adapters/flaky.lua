-- Fake adapter: the second run of a fixture writes another value than
-- the first (ORACLE_NONDETERMINISM). oracle-run writes run n to
-- runs/<fixture>/<n>.json, so run 1's file exists from run 2 on.
local common = dofile((arg[0]:gsub("[^/\\]+$", "")) .. "common.lua")
local _, output, names = common.args(...)
local first = io.open((output:gsub("[^/\\]+$", "")) .. "1.json", "rb")
local life = 1
if first then
	first:close()
	life = 2
end
common.write(output, common.done(names, '{"Life":' .. life .. "}"))
os.exit(0)
