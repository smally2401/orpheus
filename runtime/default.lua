local O = orpheus
local ui, player, util, theme = O.ui, O.player, O.util, O.theme

local function current_title()
	local t = player.current
	if not t then
        return "orpheus"
    end
	local s = t.title
	if t.artist then
        s = s .. "  \u{2014}  " .. t.artist
    end
	if t.album then
        s = s .. "  \u{00b7}  " .. t.album
    end
	return s
end

ui.add(ui.Label {
	id = "now_playing",
	x = 10, y = 6, w = -10, h = 28,
	text = current_title,
})

ui.add(ui.Label {
	id = "status",
	x = -10, y = 6, w = 260, h = 28,
	align = "right",
	fg = function() return theme.dim end,
	text = function()
		local lib = O.library
		if lib.scanning then
            return string.format("scanning %d / %d", lib.scan_done, lib.scan_total)
        end
		local v = O.view
		return string.format("%s \u{00b7} %d tracks", v.name, #v.tracks)
	end,
})

local function is_current(item)
    return item == player.current
end

ui.add(ui.List {
	id = "tracklist",
	x = 10, y = 40, w = -10, h = -64,
	header = true,
	items = function() return O.view.tracks end,
	empty_text = function()
		if O.library.scanning then
            return "Scanning\u{2026}"
        end
		return "No tracks. Drop files here or set orpheus.config.music_dirs"
	end,
	columns = {
		{ title = "#", width = 40, align = "right", get = function(t) return t.track end },
		{ title = "Title", key = "title" },
		{ title = "Artist", key = "artist" },
		{ title = "Album", key = "album" },
		{ title = "Time", width = 60, align = "right", get = function(t) return t.duration and util.format_time(t.duration) end },
	},
	row_fg = function(_, item)
		if is_current(item) then
            return theme.playing
        end
	end,
	on_activate = function(_, item)
		player.play(O.view, O.view:index_of(item))
	end,
})

local controls = ui.add(ui.Box {
	id = "controls",
	x = 0, y = "100%-56", w = 0, h = 56,
})

controls:add(ui.Button { id = "prev", x = 10, y = 12, w = 36, h = 32, icon = ui.icons.prev, action = function() player.prev() end })
controls:add(ui.Button {
	id = "play", x = 50, y = 12, w = 44, h = 32,
	icon = function(self, ...)
		if player.state() == "playing" then
            return ui.icons.pause(self, ...)
        end
		return ui.icons.play(self, ...)
	end,
	action = function() player.toggle() end,
})
controls:add(ui.Button { id = "next", x = 98, y = 12, w = 36, h = 32, icon = ui.icons.next, action = function() player.next() end })

controls:add(ui.Slider {
	id = "seek",
	x = 150, y = 18, w = -300, h = 20,
	value = function()
		local d = player.duration()
		return d > 0 and player.position() / d or 0
	end,
	on_change = function(_, v, final)
		if final then
            player.seek(v * player.duration())
        end
	end,
})

controls:add(ui.Label {
	id = "time",
	x = -180, y = 12, w = 110, h = 32,
	align = "center",
	fg = function() return theme.dim end,
	text = function()
		local seek = ui.get("seek")
		local pos = player.position()
		if seek and seek.drag_value then
            pos = seek.drag_value * player.duration()
        end
		return util.format_time(pos) .. " / " .. util.format_time(player.duration())
	end,
})

controls:add(ui.Slider {
	id = "volume",
	x = -10, y = 18, w = 100, h = 20,
	value = function() return player.volume() end,
	on_change = function(_, v) player.volume(v) end,
})

ui.bind("space", player.toggle)
ui.bind("right", function() player.seek_by(5) end)
ui.bind("left", function() player.seek_by(-5) end)
ui.bind("ctrl+right", function() player.next() end)
ui.bind("ctrl+left", function() player.prev() end)
ui.bind("=", function() player.volume(player.volume() + 0.05) end)
ui.bind("-", function() player.volume(player.volume() - 0.05) end)
ui.bind("ctrl+q", O.quit)

O.on("track_changed", function(t)
	O.window.set_title(t:display() .. " \u{00b7} orpheus")
	local list = ui.get("tracklist")
	if list and O.view == player.queue and player.index then
        list:scroll_to(player.index)
    end
end)

O.on("drop", function(path)
	O.library.scan(path)
end)
