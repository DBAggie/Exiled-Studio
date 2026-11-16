-- Fake adapter: a document for protocol 3, the one before (ORACLE_PROTOCOL).
local common = dofile((arg[0]:gsub("[^/\\]+$", "")) .. "common.lua")
local _, output, names = common.args(...)
common.write(output, common.done(names, '{"Life":1}', 3))
os.exit(0)
