local O = require("orpheus.core")
O.ui = require("orpheus.ui")
local ui, a = O.ui, O.audio
assert(a.load(O.args[1]))
a.play()
ui.add(ui.Button { x = 10, y = 10, w = 80, h = 30,
  text = function() return a.state() end,
  action = function() if a.state() == "playing" then a.pause() else a.play() end end })
ui.add(ui.Slider { x = 100, y = 15, w = -10, h = 20,
  value = function() return a.position() / math.max(1, a.duration()) end,
  on_change = function(_, v, final) if final then a.seek(v * a.duration()) end end })
ui.add(ui.Slider { x = 10, y = 50, w = 150, h = 20,
  value = function() return a.volume() end, on_change = function(_, v) a.volume(v) end })
O.on("track_end", function() O.log("finished!") end)

