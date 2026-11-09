local O = require("orpheus.core")
O.ui = require("orpheus.ui")
local ui = O.ui

ui.add(ui.Label { id = "clock", x = 10, y = 10, w = -10, h = 30,
  text = function() return os.date("%H:%M:%S") end })
ui.add(ui.Button { x = 10, y = 50, w = 120, h = 32, text = "Click me",
  action = function() O.log("clicked") end })
ui.add(ui.Button { x = -10, y = "100%-50", w = 40, h = 40, icon = ui.icons.play,
  action = function() O.log("play") end })
local box = ui.add(ui.Box {
  x = "25%", y = 100, w = "50%", h = 200, bg = "#334", border_width = 2,
  on_wheel = function(_, dy) O.log("wheel", dy) end,
  children = {
    ui.Button { x = 10, y = 10, w = 100, h = 30, text = "inner" },
    ui.Label { x = 10, y = -10, w = -10, h = 20, text = "bottom of the box" },
  },
})
ui.bind("ctrl+q", O.quit)
ui.bind("space", function() box.visible = not box.visible end)

