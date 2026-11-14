local O = require("orpheus.core")
O.ui = require("orpheus.ui")
O.playlist = require("orpheus.playlist")
O.library = require("orpheus.library")
local t0 = O.time()
O.library.rescan(function() O.log(#O.library.tracks, "tracks in", O.time() - t0, "s") end)
O.on("library_progress", function(d, t) if d % 200 == 0 then O.log(d, "/", t) end end)
O.ui.add(O.ui.List {
  x = 10, y = 10, w = -10, h = -10, header = true,
  items = function() return O.library.tracks end,
  columns = { { title = "Title", key = "title" }, { title = "Artist", key = "artist" },
              { title = "Time", width = 60, align = "right",
                get = function(t) return O.util.format_time(t.duration) end } },
  on_activate = function(_, t) O.audio.load(t.path) O.audio.play() end,
})
