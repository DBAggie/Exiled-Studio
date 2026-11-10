-- Fake adapter: a document for protocol 2, the one before (ORACLE_PROTOCOL).
local common = dofile((arg[0]:gsub("[^/\\]+$", "")) .. "common.lua")
local _, output, names = common.args(...)
common.write(output, common.done(names, '{"Life":1}', 2))
os.exit(0)
