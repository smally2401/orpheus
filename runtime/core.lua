local O = orpheus

local listeners = {}
local in_error = false

function O.error(msg)
    msg = tostring(msg)
    io.stderr:write("orpheus: lua: ", msg, "\n")
    O.last_error = msg
    if in_error then
       return
    end
    in_error = true
    O.emit("error", msg)
    in_error = false
end

function O.log(...)
    local parts = {}
    for i = 1, select("#", ...) do
        parts[i] = tostring((select(i, ...)))
    end
    io.stderr:write("orpheus: ", table.concat(parts, " "), "\n")
end

function O.on(name, fn)
    local l = listeners[name]
    if not l then
        l = {}
        listeners[name] = l
    end
    l[#l + 1] = fn
    return fn
end

function O.off(name, fn)
    local l = listeners[name]
    if not l then
        return
    end
    for i = #l, 1, -1 do
        if l[i] == fn then
            table.remove(l, i)
        end
    end
end

function O.emit(name, ...)
    local l = listeners[name]
    if not l or #l == 0 then
        return false
    end
    local snapshot = table.move(l, 1, #l, 1, {})
    for i = 1, #snapshot do
        local ok, r = xpcall(snapshot[i], debug.traceback, ...)
        if not ok then
            O.error(r)
        elseif r == true then
            return true
        end
    end
    return false
end

local timers = {}

local function add_timer(t)
    timers[#timers + 1] = t
    return t
end

function O.after(delay, fn)
    return add_timer({
        at = O.time() + delay,
        fn = fn,
    })
end

function O.every(interval, fn)
    return add_timer({
        at = O.time() + interval,
        interval = interval,
        fn = fn,
    })
end

function O.cancel(t)
    if t then
        t.cancelled = true
    end
end

local function run_timers()
    if #timers == 0 then
        return
    end
    local now = O.time()
    local due = {}
    for i = #timers, 1, -1 do
        local t = timers[i]
        if t.cancelled then
            table.remove(timers, i)
        elseif now >= t.at then
            due[#due + 1] = t
            if t.interval then
                t.at = math.max(t.at + t.interval, now)
            else
                table.remove(timers, i)
            end
        end
    end
    for i = #due, 1, -1 do
        local t = due[i]
        if not t.cancelled then
            local ok, err = xpcall(t.fn, debug.traceback, t)
            if not ok then
                O.error(err)
            end
        end
    end
end

function O._event(name, ...)
    if name == "frame" then
        local dt, visible = ...
        run_timers()
        O.emit("update", dt)
        if visible then
            O.emit("draw")
        end
    else
        O.emit(name, ...)
    end
end

local util = {}
O.util = util

function util.trim(s)
    return (s:gsub("^%s+", ""):gsub("%s+$", ""))
end

function util.split(s, sep)
    local out = {}
    for part in (s .. sep):gmatch("(.-)" .. sep:gsub("%p", "%%%0")) do
        out[#out + 1] = part
    end
    return out
end

function util.basename(p)
    return p:match("([^/]+)/*$") or p
end

function util.dirname(p)
    return p:match("^(.*)/[^/]*$") or "."
end

function util.extension(p)
    return (p:match("%.([^./]+)$") or ""):lower()
end

function util.exists(p)
    return O.fs.stat(p) ~= nil
end

function util.format_time(sec)
  if not sec or sec ~= sec or sec < 0 then
      return "--:--"
  end
  sec = math.floor(sec + 0.5)
  local h, m, s = sec // 3600, (sec // 60) % 60, sec % 60
  if h > 0 then
      return string.format("%d:%02d:%02d", h, m, s)
  end
  return string.format("%d:%02d", m, s)
end

function util.url_encode(s)
  return (tostring(s):gsub("[^%w%-%._~]", function(c) return string.format("%%%02X", c:byte()) end))
end

function util.shell_quote(s)
  return "'" .. tostring(s):gsub("'", "'\\''") .. "'"
end

function util.serialize(v, indent)
  indent = indent or ""
  local t = type(v)
  if t == "string" then
      return string.format("%q", v)
  end
  if t == "number" then
    if math.type(v) == "integer" then
        return tostring(v)
    end
    return string.format("%.17g", v)
  end
  if t == "boolean" or t == "nil" then
      return tostring(v)
  end
  if t ~= "table" then
      error("cannot serialize a " .. t)
  end
  local parts, inner = {}, indent .. "  "
  local n = #v
  for i = 1, n do
      parts[#parts + 1] = inner .. util.serialize(v[i], inner)
  end
  local keys = {}
  for k in pairs(v) do
    if not (math.type(k) == "integer" and k >= 1 and k <= n) then
        keys[#keys + 1] = k
    end
  end
  table.sort(keys, function(a, b) return tostring(a) < tostring(b) end)
  for _, k in ipairs(keys) do
    local ks = (type(k) == "string" and k:match("^[%a_][%w_]*$")) and k or "[" .. util.serialize(k) .. "]"
    parts[#parts + 1] = inner .. ks .. " = " .. util.serialize(v[k], inner)
  end
  if #parts == 0 then
      return "{}"
  end
  return "{\n" .. table.concat(parts, ",\n") .. "\n" .. indent .. "}"
end

function util.save(path, value)
  O.fs.mkdir(util.dirname(path))
  local tmp = path .. ".tmp"
  local f, err = io.open(tmp, "w")
  if not f then
      return nil, err
  end
  f:write("return ", util.serialize(value), "\n")
  f:close()
  return os.rename(tmp, path)
end

function util.load(path)
  local chunk = loadfile(path, "t", {})
  if not chunk then
      return nil
  end
  local ok, v = pcall(chunk)
  if ok then
      return v
  end
  return nil
end

function util.read_file(path)
  local f = io.open(path, "rb")
  if not f then
      return nil
  end
  local s = f:read("a")
  f:close()
  return s
end

function util.write_file(path, data)
  O.fs.mkdir(util.dirname(path))
  local f, err = io.open(path, "wb")
  if not f then
      return nil, err
  end
  f:write(data)
  f:close()
  return true
end

function O.shell(cmd, cb)
  return O.spawn({ "/bin/sh", "-c", cmd }, cb)
end

O.theme = {
    bg = "#1b1c22",
    fg = "#d9dae3",
    dim = "#8b8d9c",
    accent = "#6aa3ff",
    panel = "#23242c",
    border = "#363844",
    border_width = 0,
    button = "#2d2f39",
    button_hover = "#393c48",
    button_active = "#4a4e5e",
    selection = "#34466a",
    row_alt = "#202128",
    playing = "#8fbcff",
    error = "#ff6b6b",
    scrollbar = "#3d404c",
    font = nil,
    padding = 6,
    row_height = nil,
}

O.config = {
  music_dirs = { O.paths.music },
  extensions = { "mp3", "flac", "ogg", "oga", "wav" },
  autoscan = true,
  library_sort = "albumartist,album,disc,track,path",
  restore_session = true,
  scan_budget = 0.008,
}

return O
