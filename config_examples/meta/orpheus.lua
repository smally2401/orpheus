---@meta

---@alias HexColor string

---@class OrpheusKeymaps
---@field toggle_play string?
---@field next_track string?
---@field prev_track string?
---@field volume_up string?
---@field volume_down string?
---@field seek_forward string?
---@field seek_backward string?
---@field open_library string?
---@field open_playlists string?

---@class OrpheusPlaylist
---@field name string
---@field songs string[]
---@field sort boolean?
---@field art string?

--- A snapshot of the currently playing song, passed to `on_song_change`.
--- All three fields are always present together, there's no
--- partial/optional version of this table.
---@class OrpheusCurrentSong
---@field title string
---@field artist string
---@field album string

--- The shape `on_startup` must have if you define it.
---@alias OrpheusStartupCallback fun()

--- The shape `on_song_change` must have if you define it.
---@alias OrpheusSongChangeCallback fun(song: OrpheusCurrentSong)

--- The shape `on_song_halfway` must have if you define it.
---@alias OrpheusSongHalfwayCallback fun()

--- Path to your music library. `~` is expanded to your home directory.
---@type string
music_dir = "~/Music"

--- Volume when opening the app, 0.0-1.0
---@type number?
default_volume = 1.0

---@type boolean?
window_maximized = true

---@type number?
window_width = 800

---@type number?
window_height = 600

--- Maps an action name (e.g. "toggle_play") to a key combo string
--- (e.g. "ctrl+shift+p"). See the README for valid actions and key names.
---@type OrpheusKeymaps?
keymaps = {}

---@type OrpheusPlaylist[]?
playlists = {}

--- Recursively lists every song file under `music_dir/relative_dir`,
--- returned as paths relative to `music_dir`.
---@param relative_dir string
---@return string[]
function list_music_files(relative_dir) end

--- Changes a UI element's property at runtime, `property` must be one of
--- `"bg"`, `"text_color"`, or `"text_size"`. `value` must be a hex color
--- string (with or without `#`) for the first two, or a number for
--- `"text_size"`. Unrecognized element names, unknown properties, or
--- malformed values are logged and ignored.
---@param element string
---@param property "bg"|"text_color"|"text_size"
---@param value string|number
function set_property(element, property, value) end

--- Called once by Orpheus at startup, after config.lua has finished
--- running. Define this yourself to, for example, set initial theme 
--- colors/sizes via `set_property`.
---@type OrpheusStartupCallback
on_startup = nil

--- Called by Orpheus whenever the current track changes. Define this
--- yourself in config.lua if you want to react to song changes (e.g.
--- scrobbling, writing a "now playing" file, notifications). Not called
--- if left undefined.
---@type OrpheusSongChangeCallback?
on_song_change = nil

--- Called by Orpheus partway through the current track (roughly halfway
--- through its duration). Define this yourself if you want to react at
--- that point specifically, e.g. Last.fm-style scrobbling, which requires
--- waiting until partway through a track before submitting a scrobble.
--- Not called if left undefined.
---@type OrpheusSongHalfwayCallback?
on_song_halfway = nil
