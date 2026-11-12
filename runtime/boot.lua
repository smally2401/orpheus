local O = require("orpheus.core")
local P = require("orpheus.playlist")
local pl = P.new("test", { tracks = {
  { path = "a", title = "beta", artist = "Zed", track = 2 },
  { path = "b", title = "Alpha", artist = "Zed", track = 1 },
  { path = "c", title = "gamma", artist = "Abe", track = 10 },
} })
local function show() local t = {} for i, x in ipairs(pl.tracks) do t[i] = x.title end print(table.concat(t, ", ")) end
pl:sort("title") show()
pl:sort("-track") show()
pl:sort("artist") show()
