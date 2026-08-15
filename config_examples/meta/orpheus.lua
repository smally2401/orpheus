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

--- A snapshot of the currently playing song, passed to `on_song_change`
--- and returned by `player.current_song()`. All three fields are always
--- present together.
---@class OrpheusCurrentSong
---@field title string
---@field artist string
---@field album string

--- Live playback state, returned by `player.get_state()`. All fields are
--- always present together.
---@class OrpheusPlaybackState
---@field title string
---@field artist string
---@field album string
---@field position number Current position within the track, in seconds.
---@field duration number Total track duration, in seconds.
---@field paused boolean

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

--- Direct playback control, callable from any scripting context
--- (`on_startup`, `on_song_change`, `on_song_halfway`, timer callbacks,
--- or any function you define and call yourself).
---@class OrpheusPlayer
local OrpheusPlayer = {}

--- Resume playback.
function OrpheusPlayer.play() end

--- Pause playback.
function OrpheusPlayer.pause() end

--- Toggle between playing and paused.
function OrpheusPlayer.toggle_play() end

--- Skip to the next track in the queue.
function OrpheusPlayer.next_track() end

--- Go back to the previous track in the queue.
function OrpheusPlayer.prev_track() end

--- Jump to an absolute position (in seconds) within the current track.
---@param seconds number
function OrpheusPlayer.seek(seconds) end

--- Seek relative to the current position. Negative values seek backward.
---@param seconds number
function OrpheusPlayer.seek_by(seconds) end

--- Set volume, from 0.0 to 1.0
---@param volume number
function OrpheusPlayer.set_volume(volume) end

--- Cycle repeat mode: off -> queue -> track -> off.
function OrpheusPlayer.toggle_repeat() end

--- Toggle shuffle on/off.
function OrpheusPlayer.toggle_shuffle() end

--- Returns the current playback state, or nil if nothing's played yet
--- this session.
---@return OrpheusPlaybackState?
function OrpheusPlayer.get_state() end

---@type OrpheusPlayer
player = {}

--- Runs `fn` once, after `seconds` have passed. `seconds` accepts
--- fracntional values.
---@param seconds number
---@param fn fun()
function defer(seconds, fn) end

--- Runs `fn` repeatedly, once every `seconds`, indefinitely. `seconds`
--- accepts fractional values.
---@param seconds number
---@param fn fun()
function every(seconds, fn) end

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
