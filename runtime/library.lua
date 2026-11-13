local O = orpheus
local util = O.util
local P = require("orpheus.playlist")

local lib = {
    by_path = {},
    scanning = false,
    scan_done = 0,
    scan_total = 0,
}

lib.all = P.new("Library")
lib.tracks = lib.all.tracks

local cache, cache_dirty = nil, false

local function cache_file()
    return O.paths.cache .. "/library.lua"
end

local function load_cache()
    if cache then
        return
    end
    cache = util.load(cache_file()) or {}
    if cache.version ~= 1 then
        cache = {
            version = 1,
            files = {},
        }
    end
end

function lib.save_cache(prune)
    if not cache or not (cache_dirty or prune) then
        return
    end
    if prune then
        local files = {}
        for path, e in pairs(cache.files) do
            if lib.by_path[path] then
                files[path] = e
            end
        end
        cache.files = files
    end
    util.save(cache_file(), cache)
    cache_dirty = false
end

local Track = {}
Track.__index = Track
lib.Track = Track

local function num(s)
    if type(s) == "number" then
        return s
    end
    if type(s) ~= "string" then
        return nil
    end
    return tonumber(s:match("^%s*(%d+)"))
end

function lib.make_track(path, raw)
    raw = raw or {}
    local file = util.basename(path)
    local t = setmetatable({
        path = path,
        filename = file,
        dir = util.dirname(path),
        ext = util.extension(path),
        tags = raw,
        title = raw.title or file:gsub("%.[^.]+$", ""),
        artist = raw.artist,
        album = raw.album,
        albumartist = raw.albumartist or raw["album artist"] or raw.album_artist,
        genre = raw.genre,
        date = raw.date or raw.year,
        track = num(raw.tracknumber or raw.track),
        disc = num(raw.discnumber or raw.disc),
        duration = tonumber(raw.duration),
    }, Track)
    t.year = t.date and num(t.date:match("%d%d%d%d")) or nil
    return t
end

function Track:display()
    if self.artist then
        return self.artist .. " \u{2014} " .. self.title
    end
    return self.title
end

local function read_track(path, st)
    load_cache()
    st = st or O.fs.stat(path)
    local e = cache.files[path]
    if e and st and e.mtime == st.mtime and e.size == st.size then
        return lib.make_track(path, e.tags)
    end
    local raw = O.tags(path) or {}
    if st then
        cache.files[path] = {
            mtime = st.mtime,
            size = st.size,
            tags = raw,
        }
        cache_dirty = true
    end
    return lib.make_track(path, raw)
end

function lib.get(path)
    local t = lib.by_path[path]
    if t then
        return t
    end
    t = read_track(path)
    lib.by_path[path] = t
    return t
end

function lib.is_audio(path)
    local ext = util.extension(path)
    for _, e in ipairs(O.config.extensions) do
        if e == ext then
            return true
        end
    end
    return false
end

local in_library = {}

function lib.add(path, quiet)
    local t = lib.get(path)
    if not in_library[t] then
        in_library[t] = true
        lib.tracks[#lib.tracks + 1] = t
        if not quiet then
            lib.all:changed()
        end
    end
    return t
end

function lib.contains(t)
    return in_library[t] == true
end

function lib.remove(t)
    if not in_library[t] then
        return
    end
    in_library[t] = nil
    lib.all:remove(t)
end

function lib.clear()
    in_library = {}
    lib.all:clear()
end

function lib.matcher(query)
    local words = {}
    for w in query:lower():gmatch("%S+") do
        words[#words + 1] = w
    end
    return function(t)
        local hay = t._hay
        if not hay then
            hay = table.concat({
                t.title or "",
                t.artist or "",
                t.album or "",
                t.albumartist or "",
                t.genre or "",
            }, "\n"):lower()
            t._hay = hay
        end
        for _, w in ipairs(words) do
            if not hay:find(w, 1, true) then
                return false
            end
        end
        return true
    end
end

function lib.search(query)
    local m, out = lib.matcher(query), {}
    for _, t in ipairs(lib.tracks) do
        if m(t) then
            out[#out + 1] = t
        end
    end
    return out
end

local queue, qi, scan_cbs = {}, 1, {}
local prune_after_scan = false

local function finish_scan()
    lib.scanning = false
    queue, qi = {}, 1
    if O.config.library_sort then
        P.sort_tracks(lib.tracks, O.config.library_sort)
    end
    lib.all:changed()
    lib.save_cache(prune_after_scan)
    prune_after_scan = false
    local cbs = scan_cbs
    scan_cbs = {}
    for _, cb in ipairs(cbs) do
        cb()
    end
    O.emit("library_scanned", #lib.tracks)
end

O.on("update", function(dt)
    if not lib.scanning then
        return
    end
    local deadline = O.time() + math.max(O.config.scan_budget, math.min(dt, 0.25) * 0.5)
    local n = 0
    while qi <= #queue do
        lib.add(queue[qi], true)
        qi = qi + 1
        n = n + 1
        if n % 16 == 0 and O.time() > deadline then
            break
        end
    end
    lib.scan_done = qi - 1
    O.emit("library_progress", lib.scan_done, lib.scan_total)
    if qi > #queue then
        finish_scan()
    end
end)

function lib.scan(paths, done)
    if type(paths) == "string" then
        paths = { paths }
    end
    for _, p in ipairs(paths) do
        local st = O.fs.stat(p)
        if st and st.dir then
            for _, f in ipairs(O.fs.scan(p, O.config.extensions)) do
                queue[#queue + 1] = f
            end
        elseif st then
            queue[#queue + 1] = p
        end
    end
    if done then
        scan_cbs[#scan_cbs + 1] = done
    end
    lib.scan_total = #queue
    if not lib.scanning then
        lib.scanning = true
        O.emit("library_progress", 0, lib.scan_total)
    end
end

function lib.rescan(done)
    for i = #lib.tracks, 1, -1 do
        local t = lib.tracks[i]
        if not util.exists(t.path) then
            in_library[t] = nil
            table.remove(lib.tracks, i)
        end
    end
    prune_after_scan = true
    lib.scan(O.config.music_dirs, done)
end

O.on("quit", function()
    lib.save_cache()
end)

return lib
