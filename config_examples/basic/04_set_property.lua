-- Demonstrates set_property: changes the background color of every UI
-- region to a random color on every track change. See the config
-- README's "Scripting Hooks" section for the full reference, and
-- 05_on_song_change.lua for a simpler example using the same hook.

---@param seed number
---@return string
local function random_hex(seed)
    math.randomseed(seed)
    local r = math.random(0, 255)
    local g = math.random(0, 255)
    local b = math.random(0, 255)
    local hex = string.format("#%02x%02x%02x", r, g, b)
    return hex
end

function on_song_change()
    set_property("sidebar", "bg", random_hex(os.time()))
    set_property("now_playing_bar", "bg", random_hex(os.time() + 1))
    set_property("library_view", "bg", random_hex(os.time() + 2))
    set_property("album_view", "bg", random_hex(os.time() + 3))
    set_property("playlists_view", "bg", random_hex(os.time() + 4))
    set_property("open_playlist_view", "bg", random_hex(os.time() + 5))
end
