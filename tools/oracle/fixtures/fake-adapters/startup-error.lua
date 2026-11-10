-- Fake adapter: reports a PoB startup error (ORACLE_LAUNCH).
local common = dofile((arg[0]:gsub("[^/\\]+$", "")) .. "common.lua")
local _, output = common.args(...)
common.write(output, common.failed("startup", "prompt", "Error loading main script"))
os.exit(1)
