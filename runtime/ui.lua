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

function ui.row_height(font)
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

ui.Slider = Widget:extend {
    cursor = "hand",
    step = 0.05,

    draw = function(self, x, y, w, h)
        local v = self.drag_value or self:get("value") or 0
        v = math.max(0, math.min(1, v))
        local track = self:get("bg", "button")
        local th = math.max(2, math.floor(self:get("track_height") or h / 3))
        local ty = y + math.floor((h - th) / 2)
        gfx.rect(x, ty, w, th, track)
        gfx.rect(x, ty, math.floor(w * v), th, self:get("fill", "accent"))
        if self:is_hovered() or self.drag_value then
            local kw = math.max(4, math.floor(h / 2))
            gfx.rect(x + math.floor(w * v) - kw / 2, y + (h - h * 0.8) / 2, kw, h * 0.8, self:get("knob", "fg"))
        end
    end,

    set_from = function(self, lx, final)
        local v = math.max(0, math.min(1, lx / math.max(1, self.aw)))
        self.drag_value = (not final) and v or nil
        if self.on_change then
            self:on_change(v, final)
        end
    end,

    on_mouse_down = function (self, lx)
       self:set_from(lx, false)
    end,

    on_drag = function (self, lx)
       self:set_from(lx, false)
    end,

    on_mouse_up = function (self, lx)
       self:set_from(lx, true)
    end,

    on_wheel = function (self, dy)
        local v = (self:get("value") or 0) + dy * self.step
        if self.on_change then
            self:on_change(math.max(0, math.min(1, v)), true)
        end
    end,
}

ui.List = Widget:extend {
    framed = true,
    focusable = true,
    header = false,
    init = function(self)
        self.scroll = 0
        self.selection = {}
        self.anchor = nil
        self.cursor_index = nil
        self._gen = -1
    end,
}
local List = ui.List

function List:refresh()
    self._gen = -1
end

function List:rows()
    local src = self:get("items") or {}
    local filt = self.filter
    if self._src == src and self._gen == ui.generation and
        self._filt == filt and self._n == #src then
            return self._rows
        end
    self._src, self._gen, self._filt, self._n = src, ui.generation, filt, #src
    if filt then
        local out = {}
        for _, it in ipairs(src) do
            if filt(it) then
                out[#out + 1] = it
            end
        end
        self._rows = out
    else
        self._rows = src
    end
    return self._rows
end

function List:row_h()
    return self:get("row_height") or ui.row_height(font_of(self))
end

function List:header_h()
    return self:get("header") and self:row_h() or 0
end

function List:page()
    return math.max(1, math.floor((self.ah - self:header_h()) / self:row_h()))
end

function List:clamp_scroll()
    local rows = self:rows()
    local max = math.max(0, #rows * self:row_h() - (self.ah - self:header_h()))
    self.scroll = math.max(0, math.min(self.scroll, max))
end

function List:scroll_to(i)
    if not self.ah then
        return
    end
    local rh = self:row_h()
    local view = self.ah - self:header_h()
    local top = (i - 1) * rh
    if top < self.scroll then
        self.scroll = top
    elseif top + rh > self.scroll + view then
        self.scroll = top + rh - view
    end
    self:clamp_scroll()
end

function List:selected_items()
    local out = {}
    for _, it in ipairs(self:rows()) do
        if self.selection[it] then
            out[#out + 1] = it
        end
    end
    if #out == 0 and self.cursor_index then
        out[1] = self:rows()[self.cursor_index]
    end
    return out
end

function List:select_index(i, mode)
    local rows = self:rows()
    if #rows == 0 then
        return
    end
    i = math.max(1, math.min(#rows, i))
    local it = rows[i]
    if mode == "toggle" then
        self.selection[it] = not self.selection[it] or nil
        self.anchor = i
    elseif mode == "range" and self.anchor then
        self.selection = {}
        for k = math.min(self.anchor, i), math.max(self.anchor, i) do
            self.selection[rows[k]] = true
        end
    else
        self.selection = { [it] = true }
        self.anchor = i
    end
    self.cursor_index = i
    self:scroll_to(i)
    if self.on_select then
        self:on_select(it, i)
    end
end

function List:column_layout(w)
    local cols = self:get("columns") or { { get = tostring } }
    local fixed, flex_n = 0, 0
    for _, c in ipairs(cols) do
        local cw = c.width
        if not cw then
            flex_n = flex_n + 1
        elseif cw <= 1 then
            fixed = fixed + cw * w
        else
            fixed = fixed + cw
        end
    end
    local flex = flex_n > 0 and math.max(0, (w - fixed) / flex_n) or 0
    local out, x = {}, 0
    for i, c in ipairs(cols) do
        local cw = c.width
        cw = (not cw) and flex or (cw <= 1 and cw * w or cw)
        out[i] = {
            col = c,
            x = math.floor(x),
            w = math.floor(cw),
        }
        x = x + cw
    end
    return out
end

local function cell_text(col, item, index)
    if col.get then
        return col.get(item, index)
    end
    if col.key then
        return item[col.key]
    end
    return ""
end

function List:render_row(item, index, x, y, w, h, selected, layout)
    local font = font_of(self)
    local fg = (self.row_fg and self:row_fg(item, index)) or self:get("fg", "fg")
    local pad = self:get("padding") or theme.padding
    for _, c in ipairs(layout) do
        local t = cell_text(c.col, item, index)
        if t ~= nil and t ~= "" then
            draw_aligned(tostring(t), x + c.x, y, c.w, h,
            c.col.fg or fg, font, c.col.align or "left", pad)
        end
    end
end

function List:draw(x, y, w, h)
    gfx.rect(x, y, w, h, self:get("bg", "panel"))
	local rows = self:rows()
	local rh = self:row_h()
	local hh = self:header_h()
	local sbw = (#rows * rh > h - hh) and 8 or 0
	local layout = self:column_layout(w - sbw)
	self:clamp_scroll()
	local first = math.floor(self.scroll / rh) + 1
	local last = math.min(#rows, first + math.ceil((h - hh) / rh))
	local alt, sel = self:get("row_alt", "row_alt"), self:get("selection_bg", "selection")
	gfx.clip(x, y + hh, w, h - hh)
	for i = first, last do
		local ry = y + hh + (i - 1) * rh - self.scroll
		local it = rows[i]
		local selected = self.selection[it]
		if selected then
			gfx.rect(x, ry, w - sbw, rh, sel)
		elseif alt and i % 2 == 0 then
			gfx.rect(x, ry, w - sbw, rh, alt)
		end
		if i == self.cursor_index and self:is_focused() then
			gfx.outline(x, ry, w - sbw, rh, self:get("cursor_color", "border"), 1)
		end
		self:render_row(it, i, x, ry, w - sbw, rh, selected, layout)
	end
	gfx.unclip()
	if hh > 0 then
		gfx.rect(x, y, w, hh, self:get("header_bg", "button"))
		local pad = self:get("padding") or theme.padding
		for _, c in ipairs(layout) do
			if c.col.title then
				draw_aligned(c.col.title, x + c.x, y, c.w, hh,
                    self:get("header_fg", "dim"), font_of(self),
					c.col.align or "left", pad)
			end
		end
	end
	if sbw > 0 then
		local view = h - hh
		local total = #rows * rh
		local th = math.max(20, view * view / total)
		local ty = y + hh + (view - th) * (self.scroll / math.max(1, total - view))
		gfx.rect(x + w - sbw + 2, ty, sbw - 4, th, self:get("scrollbar", "scrollbar"))
	end
	if #rows == 0 and self.empty_text then
		draw_aligned(val(self.empty_text, self), x, y + hh, w, h - hh,
            self:get("dim", "dim"), font_of(self), "center", 0)
	end
end

function List:index_at(ly)
	local hh = self:header_h()
	if ly < hh then
        return nil
    end
	local i = math.floor((ly - hh + self.scroll) / self:row_h()) + 1
	if i >= 1 and i <= #self:rows() then
        return i
    end
end

function List:on_mouse_down(lx, ly, button, clicks)
	local rows = self:rows()
	local hh = self:header_h()
	if ly < hh then
		for _, c in ipairs(self:column_layout(self.aw)) do
			if lx >= c.x and lx < c.x + c.w and self.on_header_click then
                self:on_header_click(c.col)
            end
		end
		return
	end
	if lx > self.aw - 8 and #rows * self:row_h() > self.ah - hh then
		self.dragging_bar = true
		self:on_drag(lx, ly)
		return
	end
	local i = self:index_at(ly)
	if not i then
		if button == 1 then
            self.selection = {}
        end
		return
	end
	local ctrl, shift = O.keymods and O.keymods.ctrl, O.keymods and O.keymods.shift
	if button == 3 then
		if not self.selection[rows[i]] then
            self:select_index(i)
        end
	elseif shift then
        self:select_index(i, "range")
	elseif ctrl then
        self:select_index(i, "toggle")
	else
        self:select_index(i)
    end
	if button == 1 and clicks == 2 and self.on_activate then
        self:on_activate(rows[i], i)
    end
end

function List:on_drag(_, ly)
	if not self.dragging_bar then
        return
    end
	local hh = self:header_h()
	local total = #self:rows() * self:row_h()
	local view = self.ah - hh
	self.scroll = (ly - hh) / math.max(1, view) * total - view / 2
	self:clamp_scroll()
end

function List:on_mouse_up()
    self.dragging_bar = false
end

function List:on_right_click(lx, ly)
	local i = self:index_at(ly)
	if i and self.on_item_menu then
		self:on_item_menu(self:rows()[i], i, self.ax + lx, self.ay + ly)
	end
end

function List:on_wheel(dy)
	self.scroll = self.scroll - dy * self:row_h() * 3
	self:clamp_scroll()
end

function List:on_key(key, mods, combo)
	local rows = self:rows()
	local i = self.cursor_index or 0
	local mode = mods:find("shift") and "range" or nil
	if key == "down" then
        self:select_index(i + 1, mode)
	elseif key == "up" then
        self:select_index(math.max(1, i - 1), mode)
	elseif key == "pagedown" then
        self:select_index(i + self:page(), mode)
	elseif key == "pageup" then
        self:select_index(math.max(1, i - self:page()), mode)
	elseif key == "home" then
        self:select_index(1, mode)
	elseif key == "end" then
    self:select_index(#rows, mode)
	elseif combo == "ctrl+a" then
		self.selection = {}
		for _, it in ipairs(rows) do
            self.selection[it] = true
        end
	elseif (key == "return" or key == "keypad enter") and self.cursor_index and self.on_activate then
		self:on_activate(rows[self.cursor_index], self.cursor_index)
	else
		return false
	end
end

O.keymods = { ctrl = false, shift = false, alt = false }
local function track_mods(key, down)
	if key:find("ctrl") then
        O.keymods.ctrl = down
	elseif key:find("shift") then
        O.keymods.shift = down
	elseif key:find("alt") then
        O.keymods.alt = down end
end
O.on("key_down", function(key) track_mods(key, true) end)
O.on("key_up", function(key) track_mods(key, false) end)
O.on("focus", function(gained)
	if not gained then
        O.keymods.ctrl, O.keymods.shift, O.keymods.alt = false, false, false
    end
end)

ui.root = ui.Box {
  id = "root",
  border_width = 0,
  bg = function() return theme.bg end,
}

O.on("draw", ui.draw)

return ui
