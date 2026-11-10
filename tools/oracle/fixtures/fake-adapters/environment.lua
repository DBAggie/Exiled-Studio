-- Fake adapter: checks the environment oracle-run gives the runtime
-- (cleared, plus an allow-list) and the working directory; ok when both
-- are as expected, else a calculation error naming the difference.
local common = dofile((arg[0]:gsub("[^/\\]+$", "")) .. "common.lua")
local _, output, names = common.args(...)
local problems = {}
-- Variables that must not reach the runtime. The Rust test runs with
-- LOOM_ORACLE_DIR set, so its absence shows the environment was cleared.
for _, name in ipairs({ "LOOM_ORACLE_DIR", "CI", "PATH", "HOME", "USERPROFILE", "TEMP", "TMP", "HTTP_PROXY", "HTTPS_PROXY" }) do
	if os.getenv(name) then
		problems[#problems + 1] = name .. " is set"
	end
end
if os.getenv("LUA_PATH") ~= "./?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua" then
	problems[#problems + 1] = "LUA_PATH differs"
end
local wrapper = io.open("HeadlessWrapper.lua", "rb")
if wrapper then
	wrapper:close()
else
	problems[#problems + 1] = "working directory is not pob/src"
end
if #problems > 0 then
	common.write(output, common.failed("calculate", "adapter", table.concat(problems, "; ")))
	os.exit(1)
end
common.write(output, common.done(names, '{"Life":1}'))
os.exit(0)
