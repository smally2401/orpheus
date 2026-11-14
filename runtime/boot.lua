local O = require("orpheus.core")
O.ui = require("orpheus.ui")
O.playlist = require("orpheus.playlist")
O.library = require("orpheus.library")
O.player = require("orpheus.player")

O.view = O.library.all
function O.show(pl)
	O.view = pl or O.library.all
	O.ui.invalidate()
	O.emit("view_changed", O.view)
end

O.playlist.load_all()
require("orpheus.default")

local config = O.config_file or (O.paths.config .. "/init.lua")
O.config_file = config
if O.util.exists(config) then
	local ok, err = xpcall(dofile, debug.traceback, config)
	if not ok then O.error("config: " .. tostring(err)) end
end

if #O.args > 0 then
	O.library.scan(O.args, function()
		local first = O.args[1]
		local st = O.fs.stat(first)
		local t
		if st and not st.dir then
			t = O.library.get(first)
		else
			for _, tr in ipairs(O.library.tracks) do
				if tr.path:sub(1, #first + 1) == first .. "/" then t = tr break end
			end
		end
		if t then O.player.play(O.library.all, O.library.all:index_of(t)) end
	end)
else
	if O.config.restore_session then O.player.restore_session() end
	if O.config.autoscan then O.library.rescan() end
end

O.emit("ready")
