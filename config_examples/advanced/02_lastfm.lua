-- Demonstrates on_song_change and player.get_state() together for
-- Last.fm scrobbling: on_song_change sends a "now playing" update
-- immediately, then a periodic timer checks playback position and
-- submits the actual scrobble once the track is far enough along
-- (Last.fm's own rules require waiting until partway through a track
-- before scrobbling it).
--
-- Before this will work:
--   1. Replace API_KEY, API_SECRET, and SESSION_KEY below. Get an API key
--      and secret at https://www.last.fm/api/account/create, then use
--      Last.fm's desktop auth flow to obtain a session key.
--   2. Install the required Lua rocks, matching the same Lua version
--      Orpheus embeds (5.4):
--        luarocks --lua-version=5.4 install luasocket
--        luarocks --lua-version=5.4 install luasec
--        luarocks --lua-version=5.4 install md5
--   3. See the config README's "A note on trust" section: hooks like
--      these run with full native module access, unlike the rest of
--      config.lua, so treat any script you didn't write yourself,
--      especiall ones using `require`, with real caution.
--
-- Note the `require` calls are deferred into ensure_libs() rather than
-- sitting at the top of the file. Orpheus parses config.lua twice at
-- startup just to read plain fields like music_dir and theme, using a
-- sandboxed Lua that can't load native modules: a top-level `require`
-- here would break both of those passes. Deferring it into a function
-- body means it only actually runs when on_song_change first fires,
-- inside the seperate Lua instance dedicated to these hooks.

local API_KEY = "YOUR_LASTFM_API_KEY"
local API_SECRET = "YOUR_LASTFM_API_SECRET"
local SESSION_KEY = "YOUR_USER_SESSION_KEY"
local API_URL = "https://ws.audioscrobbler.com/2.0/"

local current_track = nil
local track_start_time = 0
local scrobbled = false

local https, ltn12, md5

local function ensure_libs()
    if not https then
        https = require("ssl.https")
        ltn12 = require("ltn12")
        md5 = require("md5")
    end
end

---@param str string
---@return string
local function url_encode(str)
    if str then
        str = str:gsub("\n", "\r\n")
        str = str:gsub("([^%w %-%_%.%~])", function(c)
            return string.format("%%%02X", string.byte(c))
        end)
        str = str:gsub(" ", "+")
    end
    return str
end

---@param params table<string, string|number>
---@param secret string
---@return string
local function generate_signature(params, secret)
    local keys = {}
    for k in pairs(params) do
        if k ~= "format" and k ~= "api_sig" then
            table.insert(keys, k)
        end
    end
    table.sort(keys)

    local sig_str = ""
    for _, k in ipairs(keys) do
        sig_str = sig_str .. k .. tostring(params[k])
    end

    sig_str = sig_str .. secret
    return md5.sumhexa(sig_str)
end

---@param params table<string, string|number>
---@return boolean success, string response
local function lastfm_post(params)
    params.api_sig = generate_signature(params, API_SECRET)
    params.format = "json"

    local body_parts = {}
    for k, v in pairs(params) do
        table.insert(body_parts, url_encode(k) .. "=" .. url_encode(tostring(v)))
    end
    local post_data = table.concat(body_parts, "&")

    local response_body = {}
    local _, status_code = https.request{
        url = API_URL,
        method = "POST",
        headers = {
            ["Content-Type"] = "application/x-www-form-urlencoded",
            ["Content-Length"] = tostring(#post_data)
        },
        source = ltn12.source.string(post_data),
        sink = ltn12.sink.table(response_body),
    }

    return status_code == 200, table.concat(response_body)
end

---@param artist string
---@param title string
---@param album string
---@return boolean success, string response
local function update_now_playing(artist, title, album)
    local params = {
        method = "track.updateNowPlaying",
        artist = artist,
        track = title,
        album = album,
        api_key = API_KEY,
        sk = SESSION_KEY,
    }

    return lastfm_post(params)
end

---@param artist string
---@param title string
---@param album string
---@return boolean success, string response
local function scrobble(artist, title, album, timestamp)
    local params = {
        method = "track.scrobble",
        artist = artist,
        track = title,
        timestamp = timestamp,
        album = album,
        api_key = API_KEY,
        sk = SESSION_KEY,
    }

    return lastfm_post(params)
end

function on_song_change(song)
    ensure_libs()

    current_track = song
    track_start_time = os.time()
    scrobbled = false

    print(string.format("[Last.fm] Now Playing: %s - %s", song.artist, song.title))

    local ok, err = update_now_playing(song.artist, song.title, song.album)
    if not ok then
        print("[Last.fm] Failed to update Now Playing:", err)
    end
end

function on_startup()
    every(5, function()
        if scrobbled or not current_track then
            return
        end

        local state = player.get_state()
        if not state or state.duration == 0 then
            return
        end

        if state.position >= (state.duration / 2) then
            scrobbled = true
            print(string.format("[Last.fm] Scrobbling: %s - %s", current_track.artist, current_track.title))

            local ok, err = scrobble(
                current_track.artist,
                current_track.title,
                current_track.album,
                track_start_time
            )

            if not ok then
                print("[Last.fm] Scrobble failed:", err)
            end
        end
    end)
end
