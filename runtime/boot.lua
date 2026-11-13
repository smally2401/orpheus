local O = require("orpheus.core")
O.ui = require("orpheus.ui")
O.playlist = require("orpheus.playlist")
O.library = require("orpheus.library")
local t0 = O.time()
O.library.rescan(function() O.log(#O.library.tracks, "tracks in", O.time() - t0, "s") end)
O.on("library_progress", function(d, t) if d % 200 == 0 then O.log(d, "/", t) end end)

