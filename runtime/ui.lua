local O = orpheus
local gfx = O.gfx
local theme = O.theme

local ui = {
    by_id = {},
    overlays = {},
    bindings = {},
    generation = 0,
    focus = nil,
    hover = nil,
    pressed = nil,
    mouse_x = 0,
    mouse_y = 0,
}

function ui.invalidate()
    ui.generation = ui.generation + 1
end

local function val(v, self)
    if type(v) == "function" then
        return v(self)
    end
    return v
end
ui.value = val

local function font_of(self)
    return self:get("font") or theme.font
end

function ui.row_height()
    return theme.row_height or math.floor(gfx.line_height(font or theme.font) + 6)
end

local Widget = {}
Widget.__index = Widget
ui.Widget = Widget

local function construct(cls, props)
  local w = setmetatable(props or {}, cls)
  local kids = w.children or {}
  w.children = {}
  for _, c in ipairs(kids) do
      w:add(c)
  end
  w:init()
  return w
end

setmetatable(Widget, { __call = construct })

function Widget:extend(def)
  local cls = def or {}
  cls.__index = cls
  cls.super = self
  return setmetatable(cls, { __index = self, __call = construct })
end

function Widget:init() end

function Widget:get(key, theme_key)
  local v = self[key]
  if v == nil and theme_key then
      v = theme[theme_key]
  end
  if type(v) == "function" then
      return v(self)
  end
  return v
end

local function register(w)
  if w.id then
      ui.by_id[w.id] = w
  end
  for _, c in ipairs(w.children) do
      register(c)
  end
end

local function unregister(w)
  if w.id and ui.by_id[w.id] == w then
      ui.by_id[w.id] = nil
  end
  if ui.focus == w then
      ui.set_focus(nil)
  end
  if ui.hover == w then
      ui.hover = nil
  end
  if ui.pressed == w then
      ui.pressed = nil
  end
  for _, c in ipairs(w.children) do
      unregister(c)
  end
end

local function attached(w)
  while w do
    if w == ui.root then
        return true
    end
    if w._overlay then
        return true
    end
    w = w.parent
  end
  return false
end

function Widget:add(child, index)
  if child.parent then
      child:remove()
  end
  child.parent = self
  if index then
    table.insert(self.children, index, child)
  else
    self.children[#self.children + 1] = child
  end
  if attached(self) then
      register(child)
  end
  return child
end

function Widget:remove()
  local p = self.parent
  if not p then
      return self
  end
  for i, c in ipairs(p.children) do
    if c == self then
        table.remove(p.children, i)
        break
    end
  end
  self.parent = nil
  unregister(self)
  return self
end

function Widget:raise()
  local p = self.parent
  if p then
      self:remove()
      p:add(self)
  end
  return self
end

function Widget:lower()
  local p = self.parent
  if p then
      self:remove()
      p:add(self, 1)
  end
  return self
end

function Widget:find(id)
  for _, c in ipairs(self.children) do
    if c.id == id then
        return c
    end
    local r = c:find(id)
    if r then
        return r
    end
  end
end

function Widget:contains(x, y)
  return self._drawn == ui.frame and x >= self.ax and y >= self.ay
     and x < self.ax + self.aw and y < self.ay + self.ah
end

function Widget:is_focused() return ui.focus == self end
function Widget:is_hovered() return ui.hover == self end
function Widget:is_pressed() return ui.pressed == self end

function Widget:draw(x, y, w, h)
  local bg = self:get("bg")
  if bg then
      gfx.rect(x, y, w, h, bg)
  end
  self:render(x, y, w, h)
end

function Widget:render() end

function Widget:draw_border(x, y, w, h)
  local bw = self:get("border_width")
  if bw == nil and self.framed then
      bw = theme.border_width
  end
  if bw and bw > 0 then
    gfx.outline(x, y, w, h, self:get("border", "border"), bw)
  end
end

local function dim(v, self, total, pos)
  if type(v) == "function" then
      v = v(self, total)
  end
  if type(v) == "string" then
    local pct, off = v:match("^(%-?[%d.]+)%%([+-]?[%d.]*)$")
    if not pct then
        error("bad geometry value '" .. v .. "'")
    end
    return total * tonumber(pct) / 100 + (tonumber(off) or 0)
  end
  return v or 0
end

function Widget:layout(pw, ph)
  local function axis(pos, size, total)
    local p = type(pos) == "function" and pos(self, pw, ph) or pos
    local s = type(size) == "function" and size(self, pw, ph) or size
    p = dim(p, self, total)
    s = dim(s, self, total)
    if s <= 0 and type(size) ~= "string" then
      if p >= 0 then
          return p, math.max(0, total - p + s)
      end
      return 0, math.max(0, total + s)
    end
    if p < 0 then
        return total - s + p, s
    end
    return p, s
  end
  local x, w = axis(self.x, self.w, pw)
  local y, h = axis(self.y, self.h, ph)
  return math.floor(x), math.floor(y), math.floor(w), math.floor(h)
end

local function draw_tree(w, px, py, pw, ph)
  local vis = w.visible
  if vis == false or (type(vis) == "function" and not vis(w)) then
      return
  end
  local x, y, ww, hh = w:layout(pw, ph)
  x, y = px + x, py + y
  w.ax, w.ay, w.aw, w.ah = x, y, ww, hh
  w._drawn = ui.frame
  if w.update then
      w:update(ww, hh)
  end
  local clip = w.clip ~= false
  if clip then
      gfx.clip(x, y, ww, hh)
  end
  w:draw(x, y, ww, hh)
  for _, c in ipairs(w.children) do
      draw_tree(c, x, y, ww, hh)
  end
  if w.draw_over then
      w:draw_over(x, y, ww, hh)
  end
  w:draw_border(x, y, ww, hh)
  if clip then
      gfx.unclip()
  end
end

ui.frame = 0

function ui.draw()
  ui.frame = ui.frame + 1
  local W, H = O.window.size()
  draw_tree(ui.root, 0, 0, W, H)
  for _, o in ipairs(ui.overlays) do
      draw_tree(o, 0, 0, W, H)
  end
end

local function hit_in(w, x, y)
  if not w:contains(x, y) then
      return nil
  end
  for i = #w.children, 1, -1 do
    local r = hit_in(w.children[i], x, y)
    if r then
        return r
    end
  end
  if w.pass_through then
      return nil
  end
  return w
end

function ui.hit(x, y)
  for i = #ui.overlays, 1, -1 do
    local o = ui.overlays[i]
    local r = hit_in(o, x, y)
    if r then
        return r
    end
    if o.modal ~= false then return o end
  end
  return hit_in(ui.root, x, y)
end

local function bubble(w, method, ...)
  while w do
    local fn = w[method]
    if fn then
      local r = fn(w, ...)
      if r ~= false then
          return true, w
      end
    end
    w = w.parent
  end
  return false
end
ui.bubble = bubble

function ui.set_focus(w)
  if ui.focus == w then
      return
  end
  local old = ui.focus
  ui.focus = w
  if old and old.on_blur then
      old:on_blur()
  end
  if w and w.on_focus then
      w:on_focus()
  end
  O.window.text_input(w ~= nil and w.wants_text == true)
end

local function local_xy(w, x, y)
    return x - (w.ax or 0), y - (w.ay or 0)
end

local function close_overlays_outside(x, y)
  for i = #ui.overlays, 1, -1 do
    local o = ui.overlays[i]
    if o:contains(x, y) then
        break
    end
    if o.dismissable == false then
        break
    end
    ui.close(o)
  end
end

O.on("mouse_down", function(x, y, button, clicks)
  close_overlays_outside(x, y)
  local t = ui.hit(x, y)
  ui.pressed = t
  local f = t
  while f and not f.focusable do
      f = f.parent
  end
  ui.set_focus(f)
  if t then
    local lx, ly = local_xy(t, x, y)
    bubble(t, "on_mouse_down", lx, ly, button, clicks)
  end
end)

O.on("mouse_up", function(x, y, button, clicks)
  local p = ui.pressed
  ui.pressed = nil
  if not p then
      return
  end
  local lx, ly = local_xy(p, x, y)
  bubble(p, "on_mouse_up", lx, ly, button)
  if p:contains(x, y) then
    if button == 3 then
      bubble(p, "on_right_click", lx, ly)
    else
      bubble(p, "on_click", lx, ly, button, clicks)
    end
  end
end)

O.on("mouse_move", function(x, y)
  ui.mouse_x, ui.mouse_y = x, y
  local t = ui.hit(x, y)
  if t ~= ui.hover then
    local old = ui.hover
    ui.hover = t
    if old and old.on_leave then
        old:on_leave()
    end
    if t and t.on_enter then
        t:on_enter()
    end
    O.window.cursor((t and t:get("cursor")) or "arrow")
  end
  if ui.pressed then
    bubble(ui.pressed, "on_drag", local_xy(ui.pressed, x, y))
  elseif t then
    bubble(t, "on_mouse_move", local_xy(t, x, y))
  end
end)

O.on("mouse_leave", function()
  if ui.hover and ui.hover.on_leave then
      ui.hover:on_leave()
  end
  ui.hover = nil
end)

O.on("wheel", function(dx, dy, x, y)
  local t = ui.hit(x, y)
  if t then
      bubble(t, "on_wheel", dy, dx)
  end
end)

local function combo_of(key, mods)
  if mods and mods ~= "" then
      return mods .. "+" .. key
  end
  return key
end

O.on("key_down", function(key, mods, rep)
  local combo = combo_of(key, mods)
  local top = ui.overlays[#ui.overlays]
  if ui.focus and bubble(ui.focus, "on_key", key, mods, combo, rep) then
      return
  end
  if top and key == "escape" and top.dismissable ~= false then
    ui.close(top)
    return
  end
  if top and top.modal ~= false then
      return
  end
  local b = ui.bindings[combo]
  if b
      then b(key, mods, rep)
      end
end)

O.on("text_input", function(text)
  if ui.focus then
      bubble(ui.focus, "on_text", text)
  end
end)

function ui.bind(combo, fn)
    ui.bindings[combo] = fn
end

function ui.unbind(combo)
    ui.bindings[combo] = nil
end

function ui.add(w, parent)
    return (parent or ui.root):add(w)
end

function ui.get(id)
    return ui.by_id[id]
end

function ui.remove(w)
  if type(w) == "string" then
      w = ui.by_id[w]
  end
  if w then
      w:remove()
  end
  return w
end

function ui.clear()
  for i = #ui.root.children, 1, -1 do
      ui.root.children[i]:remove()
  end
end

function ui.open(o)
  o._overlay = true
  ui.overlays[#ui.overlays + 1] = o
  register(o)
  return o
end

function ui.close(o)
  for i, v in ipairs(ui.overlays) do
    if v == o then
      table.remove(ui.overlays, i)
      unregister(o)
      if o.on_close then
          o:on_close()
      end
      return
    end
  end
end

ui.Box = Widget:extend { framed = true }

local function draw_aligned(text, x, y, w, h, color, font, align, pad)
  text = gfx.fit(text, w - pad * 2, font)
  local tw, th = gfx.measure(text, font)
  local tx = x + pad
  if align == "center" then
      tx = x + (w - tw) / 2
  elseif align == "right" then
      tx = x + w - tw - pad
  end
  gfx.text(text, math.floor(tx), math.floor(y + (h - th) / 2), color, font)
end
ui.draw_text = draw_aligned

ui.Label = Widget:extend {
  align = "left",
  render = function(self, x, y, w, h)
    local text = self:get("text")
    if text == nil then
        return
    end
    draw_aligned(tostring(text), x, y, w, h, self:get("fg", "fg"), font_of(self),
      self:get("align"), self:get("padding") or 0)
  end,
}

ui.Button = Widget:extend {
  align = "center",
  cursor = "hand",
  draw = function(self, x, y, w, h)
    local bg
    if self:is_pressed() and self:contains(ui.mouse_x, ui.mouse_y) then
      bg = self:get("bg_active", "button_active")
    elseif self:get("active") then
      bg = self:get("bg_active", "button_active")
    elseif self:is_hovered() then
      bg = self:get("bg_hover", "button_hover")
    else
      bg = self:get("bg", "button")
    end
    if bg then
        gfx.rect(x, y, w, h, bg)
    end
    self:render(x, y, w, h)
  end,
  render = function(self, x, y, w, h)
    if self.icon then
        return self:icon(x, y, w, h, self:get("fg", "fg"))
    end
    local text = self:get("text")
    if text then
      draw_aligned(tostring(text), x, y, w, h, self:get("fg", "fg"), font_of(self),
        self:get("align"), self:get("padding") or theme.padding)
    end
  end,
  on_click = function(self, _, _, button)
    if self.action then
        self:action(button)
    end
  end,
}

ui.icons = {}

function ui.icons.play(_, x, y, w, h, c)
  local s = math.min(w, h) * 0.4
  local cx, cy = x + w / 2, y + h / 2
  gfx.poly({ cx - s * 0.6, cy - s, cx + s * 0.9, cy, cx - s * 0.6, cy + s }, c)
end

function ui.icons.pause(_, x, y, w, h, c)
  local s = math.min(w, h) * 0.4
  local cx, cy = x + w / 2, y + h / 2
  gfx.rect(cx - s * 0.8, cy - s, s * 0.55, s * 2, c)
  gfx.rect(cx + s * 0.25, cy - s, s * 0.55, s * 2, c)
end

function ui.icons.next(_, x, y, w, h, c)
  local s = math.min(w, h) * 0.35
  local cx, cy = x + w / 2, y + h / 2
  gfx.poly({ cx - s, cy - s, cx + s * 0.5, cy, cx - s, cy + s }, c)
  gfx.rect(cx + s * 0.5, cy - s, s * 0.35, s * 2, c)
end

function ui.icons.prev(_, x, y, w, h, c)
  local s = math.min(w, h) * 0.35
  local cx, cy = x + w / 2, y + h / 2
  gfx.poly({ cx + s, cy - s, cx + s, cy + s, cx - s * 0.5, cy }, c)
  gfx.rect(cx - s * 0.85, cy - s, s * 0.35, s * 2, c)
end

function ui.icons.stop(_, x, y, w, h, c)
  local s = math.min(w, h) * 0.3
  gfx.rect(x + w / 2 - s, y + h / 2 - s, s * 2, s * 2, c)
end

ui.root = ui.Box {
  id = "root",
  border_width = 0,
  bg = function() return theme.bg end,
}

O.on("draw", ui.draw)

return ui
