local O = orpheus
local util = O.util

local P = {
    lists = {},
    order = {},
}

local Playlist = {}
Playlist.__index = Playlist
P.Playlist = Playlist

local function dir()
    return O.paths.data .. "/playlists"
end

local function file_for(name)
    return dir() .. "/" .. name:gsub("[/%z]", "_") .. ".m3u"
end

function P.new(name, opts)
    opts = opts or {}
    return setmetatable({
        name = name,
        tracks = opts.tracks or {},
        persistent = opts.persistent or false,
    }, Playlist)
end

function Playlist:changed()
    O.emit("playlist_changed", self)
    if O.ui then
        O.ui.invalidate()
    end
    if self.persistent and not self._save_timer then
        self._save_timer = O.after(0.5, function ()
            self._save_timer = nil
            self:save()
        end)
    end
end

function Playlist:len()
    return #self.tracks
end

function Playlist:len()
    return #self.tracks
end

function Playlist:index_of(track)
    for i, t in ipairs(self.tracks) do
        if t == track then
            return i
        end
    end
end

function Playlist:contains(track)
    return self:index_of(track) ~= nil
end

function Playlist:add(what, pos, opts)
    local list = what.path and { what } or what
    local unique = (opts and opts.unique) or self.unique
    local seen
    if unique then
        seen = {}
        for _, t in ipairs(self.tracks) do
            seen[t] = true
        end
    end
    pos = pos or #self.tracks + 1
    local added = 0
    for _, t in ipairs(list) do
        if not (seen and seen[t]) then
            table.insert(self.tracks, pos + added, t)
            added = added + 1
            if seen then
                seen[t] = true
            end
        end
    end
    if added > 0 then
        self:changed()
    end
    return added
end

function Playlist:remove(what)
    if type(what) == "number" then
        table.remove(self.tracks, what)
    else
        local set = {}
        if what.path then
            set[what] = true
        else
            for _, t in ipairs(what) do
                set[t] = true
            end
        end
        for i = #self.tracks, 1, -1 do
            if set[self.tracks[i]] then
                table.remove(self.tracks, 1)
            end
        end
    end
    self:changed()
end

function Playlist:move(from, to)
    local t = table.remove(self.tracks, from)
    if t then
        table.insert(self.tracks, math.max(1, math.min(to, #self.tracks + 1)), t)
        self:changed()
    end
end

function Playlist:clear()
    local t = self.tracks
    for i = #t, 1, -1 do
        t[i] = nil
    end
    self:changed()
end

function Playlist:shuffle()
    local t = self.tracks
    for i = #t, 2, -1 do
        local j = math.random(i)
        t[i], t[j] = t[j], t[i]
    end
    self:changed()
end

function Playlist:filter(pred, name)
    local out = {}
    for _, t in ipairs(self.tracks) do
        if pred(t) then
            out[#out + 1] = t
        end
    end
    return P.new(name or self.name, { tracks = out })
end

function Playlist:copy(name)
    return P.new(name or self.name,{
        tracks = table.move(self.tracks, 1, #self.tracks, 1, {})
    })
end

P.sort_keys = {
    artist = function (t)
       return t.artist or ""
    end,
    albumartist = function (t)
        return t.albumartist or t.artist or ""
    end,
    year = function (t)
        return t.year or 0
    end,
    random = function ()
        return math.random()
    end,
}

local function key_fn(name)
    return P.sort_keys[name] or function(t)
        local v = t[name]
        if v == nil then
            return ""
        end
        return v
    end
end

local function cmp_values(a, b)
    local ta, tb = type(a), type(b)
    if ta == "string" and tb == "string" then
        local la, lb = a:lower(), b:lower()
        if la ~= lb then
            return la < lb and -1 or 1
        end
        return 0
    end
    if ta == "number" and tb == "number" then
        if a ~= b then
            return a < b and -1 or 1
        end
        return 0
    end
    if ta ~= tb then
        return ta == "number" and -1 or 1
    end
    return 0
end

function P.comparator(spec)
    if type(spec) == "function" then
        return spec
    end
    if type(spec) == "string" then
        spec = util.split((spec:gsub("%s", "")), ",")
    end
    local keys = {}
    for i, k in ipairs(spec) do
        local desc = k:sub(1, 1) == "-"
        local name = desc and k:sub(2) or k
        keys[i] = {
            fn = key_fn(name),
            desc = desc,
        }
    end
    return function(a, b)
        for _, k in ipairs(keys) do
            local c = cmp_values(k.fn(a), k.fn(b))
            if c ~= 0 then
                if k.desc then
                    return c > 0
                end
                return c < 0
            end
        end
        return false
    end
end

function P.sort_tracks(tracks, spec)
    local less = P.comparator(spec)
    local pos = {}
    for i, t in ipairs(tracks) do
        pos[t] = pos[t] or i
    end
    table.sort(tracks, function(a, b)
        if less(a, b) then
            return true
        end
        if less (b, a) then
            return false
        end
        return pos[a] < pos[b]
    end)
    return tracks
end

function Playlist:sort(spec)
    P.sort_tracks(self.tracks, spec)
    self.sorted_by = type(spec) == "string" and spec or nil
    self:changed()
end

function Playlist:save()
    if not self.persistent then
        return
    end
    local lines = { "#EXTM3U" }
    for _, t in ipairs(self.tracks) do
        lines[#lines + 1] = string.format("#EXTINF:%d,%s - %s",
            math.floor(t.duration or -1), t.artist or "", t.title or "")
        lines[#lines + 1] = t.path
    end
    local ok, err = util.write_file(file_for(self.name),
        table.concat(lines, "\n") .. "\n")
    if not ok then
        O.error("saving playlist '" .. self.name .. "': " .. tostring(err))
    end
end

function P.read_m3u(path, name)
    local data = util.read_file(path)
    if not data then
        return nil
    end
    local base = util.dirname(path)
    local pl = P.new(name or util.basename(path):gsub("%.m3u8?$", ""))
    for line in data:gmatch("[^\r\n]+") do
        line = util.trim(line)
        if line ~= "" and line:sub(1, 1) ~= "#" then
            if line:sub(1, 1) ~= "/" then
                line = base .. "/" .. line
            end
            if util.exists(line) then
                pl.tracks[#pl.tracks + 1] = O.library.get(line)
            end
        end
    end
    return pl
end

function P.all()
    local out = {}
    for _, name in ipairs(P.order) do
        out[#out + 1] = P.lists[name]
    end
    return out
end

function P.get(name)
    return P.lists[name]
end

local function registered()
    O.emit("playlists_changed")
    if O.ui then
        O.ui.invalidate()
    end
end

function P.create(name, tracks)
    if P.lists[name] then
        return P.lists[name]
    end
    local pl = P.new(name, {
        persistent = true,
        tracks = tracks,
    })
    P.lists[name] = pl
    P.order[#P.order + 1] = name
    pl:save()
    registered()
    return pl
end

function P.delete(name)
    local pl = P.lists[name]
    if not pl then
        return
    end
    P.lists[name] = nil
    for i, n in ipairs(P.order) do
        if n == name then
            table.remove(P.order, i)
            break
        end
    end
    os.remove(file_for(name))
    O.cancel(pl._save_timer)
    pl.persistent = false
    registered()
end

function P.rename(name, new)
    local pl = P.lists[name]
    if not pl or P.lists[new] then
        return nil
    end
    os.remove(file_for(name))
    P.lists[name], P.lists[new] = nil, pl
    for i, n in ipairs(P.order) do
        if n == name then
            P.order[i] = new
        end
    end
    pl.name = new
    pl:save()
    registered()
    return pl
end

function P.load_all()
    local entries = O.fs.list(dir()) or {}
    for _, e in ipairs(entries) do
        if not e.dir and e.name:match("%.m3u8?$") then
            local pl = P.read_m3u(dir() .. "/" .. e.name)
            if pl and not P.lists[pl.name] then
                pl.persistent = true
                P.lists[pl.name] = pl
                P.order[#P.order + 1] = pl.name
            end
        end
    end
    registered()
end

function P.save_all()
    for _, pl in ipairs(P.all()) do
        if pl._save_timer then
            O.cancel(pl._save_timer)
            pl._save_timer = nil
            pl:save()
        end
    end
end

O.on("quit", P.save_all)

return P
