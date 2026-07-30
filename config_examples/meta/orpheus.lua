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

---@class OrpheusBg
---@field sidebar HexColor?
---@field now_playing_bar HexColor?
---@field library_view HexColor?
---@field album_view HexColor?
---@field playlists_view HexColor?
---@field open_playlist_view HexColor?

---@class OrpheusTextColor
---@field sidebar HexColor?
---@field now_playing_song HexColor?
---@field now_playing_artist HexColor?
---@field detail_view_header_title HexColor?
---@field detail_view_header_subtitle HexColor?
---@field library_list_title HexColor?
---@field library_list_subtitle HexColor?
---@field album_list_title HexColor?
---@field album_list_subtitle HexColor?

---@class OrpheusTextSize
---@field sidebar integer?
---@field now_playing_song integer?
---@field now_playing_artist integer?
---@field detail_view_header_title integer?
---@field detail_view_header_subtitle integer?
---@field library_list_title integer?
---@field library_list_subtitle integer?
---@field album_list_title integer?
---@field album_list_subtitle integer?

---@class OrpheusTheme
---@field bg OrpheusBg?
---@field text_color OrpheusTextColor?
---@field text_size OrpheusTextSize?

---@class OrpheusPlaylist
---@field name string
---@field songs string[]
---@field sort boolean?
---@field art string?

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

---@type OrpheusTheme?
theme = {}

---@type OrpheusPlaylist[]?
playlists = {}

--- Recursively lists every song file under `music_dir/relative_dir`,
--- returned as paths relative to `music_dir`.
---@param relative_dir string
---@return string[]
function list_music_files(relative_dir) end
