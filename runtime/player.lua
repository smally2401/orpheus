local O = orpheus
local audio = O.audio
local P = require("orpheus.playlist")

local player = {
    queue = nil,
    index = nil,
    current = nil,
    shuffle = false,
    repeat_mode = "off",
    started_at = nil,
    history = {},
}

local function set_state()
    O.emit("state_changed", audio.state())
end

local function sync_index()
    local q = player.queue
    if not q or not player.current then
        return
    end
    if q.tracks[player.index or 0] ~= player.current then
        player.index = q:index_of(player.current) or player.index
    end
end

local failures = 0
local function start(index)
    local q = player.queue
	local t = q and q.tracks[index]
	if not t then
        return false
    end
	local ok, err = audio.load(t.path)
	if not ok then
		O.error("cannot play " .. t.path .. ": " .. tostring(err))
		failures = failures + 1
		if failures < 10 and index < #q.tracks then
            return start(index + 1)
        end
		failures = 0
		player.current, player.index = nil, nil
		set_state()
		return false
	end
	failures = 0
	audio.play()
	player.current, player.index = t, index
	player.started_at = O.unix_time()
	local d = audio.duration()
	if d and d > 0 then
        t.duration = d
    end
	O.emit("track_changed", t, q, index)
	set_state()
	return true
end

function player.play(what, index)
	if what == nil then
		if audio.state() == "paused" or (audio.state() == "stopped" and player.current) then
			audio.play()
			set_state()
			return true
		end
		if player.queue then
            return start(player.index or 1)
        end
		return false
	end
	if getmetatable(what) == P.Playlist then
		player.queue = what
		player.history = {}
		return start(index or 1)
	end
	local q = player.queue
	local i = q and q:index_of(what)
	if not i then
		q = P.new("Queue", { tracks = { what } })
		q.temporary = true
		player.queue, i = q, 1
	end
	return start(i)
end

function player.pause()
	audio.pause()
	set_state()
end

function player.toggle()
	if audio.state() == "playing" then
        player.pause()
    else
        player.play()
    end
end

function player.stop()
	audio.stop()
	set_state()
end

function player.next(auto)
	local q = player.queue
	if not q or #q.tracks == 0 then
        return false
    end
	sync_index()
	if auto and player.repeat_mode == "one" and player.index then
        return start(player.index)
    end
	local i
	if player.shuffle and #q.tracks > 1 then
		repeat i = math.random(#q.tracks) until i ~= player.index
		if player.current then
            table.insert(player.history, player.current)
        end
	else
		i = (player.index or 0) + 1
		if i > #q.tracks then
			if player.repeat_mode ~= "all" then
				if auto then
					player.stop()
					O.emit("queue_finished", q)
				end
				return false
			end
			i = 1
		end
	end
	return start(i)
end

function player.prev()
	local q = player.queue
	if not q then
        return false
    end
	if audio.position() > 3 then
		audio.seek(0)
		return true
	end
	sync_index()
	if player.shuffle and #player.history > 0 then
		local t = table.remove(player.history)
		local i = q:index_of(t)
		if i then
            return start(i)
        end
	end
	local i = (player.index or 2) - 1
	if i < 1 then
		if player.repeat_mode ~= "all" then
            audio.seek(0)
            return true
        end
		i = #q.tracks
	end
	return start(i)
end

function player.seek(sec)
	if audio.seek(sec) then
        O.emit("seeked", sec)
    end
end

function player.seek_by(delta)
    player.seek(math.max(0, audio.position() + delta))
end

function player.volume(v)
	if v ~= nil then
		audio.volume(v)
		O.emit("volume_changed", audio.volume())
	end
	return audio.volume()
end

function player.set_shuffle(on)
	player.shuffle = on and true or false
	O.emit("mode_changed")
end

function player.cycle_repeat()
	local nxt = { off = "all", all = "one", one = "off" }
	player.repeat_mode = nxt[player.repeat_mode] or "off"
	O.emit("mode_changed")
end

function player.state()
    return audio.state()
end

function player.position()
    return audio.position()
end

function player.duration()
	local d = audio.duration()
	if d and d > 0 then
        return d
    end
	return player.current and player.current.duration or 0
end

function player.now()
	local t = player.current
	if not t then
        return nil
    end
	return {
		track = t,
		path = t.path,
		title = t.title,
		artist = t.artist,
		album = t.album,
		albumartist = t.albumartist,
		tracknumber = t.track,
		duration = player.duration(),
		position = audio.position(),
		state = audio.state(),
		started_at = player.started_at,
		queue = player.queue and player.queue.name,
		index = player.index,
	}
end

O.on("track_end", function() player.next(true) end)

O.on("library_scanned", function()
	local q = player.queue
	if q and q.temporary and player.current and O.library.contains(player.current) then
		player.queue = O.library.all
		player.index = O.library.all:index_of(player.current)
	end
end)

local function session_file() return O.paths.data .. "/session.lua" end

function player.save_session()
	local s = {
		volume = audio.volume(),
		shuffle = player.shuffle,
		repeat_mode = player.repeat_mode,
	}
	if player.current then
		s.path = player.current.path
		s.position = audio.position()
		if player.queue and player.queue.persistent then
            s.queue = player.queue.name
        end
	end
	O.util.save(session_file(), s)
end

function player.restore_session()
	local s = O.util.load(session_file())
	if type(s) ~= "table" then
        return
    end
	if s.volume then
        audio.volume(s.volume)
    end
	player.shuffle = s.shuffle or false
	player.repeat_mode = s.repeat_mode or "off"
	if s.path and O.util.exists(s.path) then
		local q = (s.queue and P.get(s.queue)) or O.library.all
		local t = O.library.get(s.path)
		local i = q:index_of(t)
		if not i then
			q = P.new("Queue", { tracks = { t } })
			q.temporary = true
			i = 1
		end
		player.queue = q
		if audio.load(t.path) then
			player.current, player.index = t, i
			if s.position then
                audio.seek(s.position)
            end
			O.emit("track_changed", t, q, i)
			set_state()
		end
	end
	O.emit("mode_changed")
end

O.on("quit", function()
	if O.config.restore_session then
        player.save_session()
    end
end)

return player
