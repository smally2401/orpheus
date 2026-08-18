//! Keybindings: the `keymaps` block of `config.lua`, plus the `KeyCombo`/
//! `KeyAction` types the rest of the app dispatches on. `VALID_KEYS` and
//! `key_string_to_key_name` (used by `KeyCombo::from_key`) come from the
//! `define_keys!` macro invocation at the bottom of this file, which is the
//! single source of truth for which Slint `Key` variants are supported.

use crate::player_bridge::PlayerCommand;
use std::collections::HashMap;

/// An action the user can trigger via a keybinding, dispatched on by
/// `main.rs`'s input handling.
#[derive(Clone, Copy)]
pub enum KeyAction {
    TogglePlay,
    NextTrack,
    PrevTrack,
    VolumeUp,
    VolumeDown,
    SeekForward,
    SeekBackward,
    OpenLibrary,
    OpenPlaylists,
}

impl KeyAction {
    #[must_use]
    pub fn to_command(self) -> Option<PlayerCommand> {
        use crate::player_bridge::PlayerCommand::*;

        match self {
            Self::TogglePlay => Some(TogglePlay),
            Self::NextTrack => Some(NextTrack),
            Self::PrevTrack => Some(PrevTrack),
            Self::VolumeUp => Some(VolumeUp),
            Self::VolumeDown => Some(VolumeDown),
            Self::SeekForward => Some(SeekForward),
            Self::SeekBackward => Some(SeekBackward),
            Self::OpenLibrary | Self::OpenPlaylists => None,
        }
    }
}

/// A single key combination, e.g. `ctrl+shift+P`. Used as the key type in
/// the `keymaps` `HashMap`, so two combos are equal only if every modifier
/// matches exactly.
#[derive(Eq, Hash, PartialEq, Clone)]
pub struct KeyCombo {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyCombo {
    /// Parses a `+`-separated combo string like `"ctrl+shift+P"` into a
    /// `KeyCombo`. Modifier names are case-insensitive and the remaining,
    /// non-modifier token is the key itself and must appear in
    /// `VALID_KEYS`, or this returns `None`.
    fn from_key(key: &str) -> Option<Self> {
        let mut final_key = String::new();
        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;

        for part in key.split('+') {
            match part.to_ascii_lowercase().as_str() {
                "control" | "ctrl" => ctrl = true,
                "shift" => shift = true,
                "alt" => alt = true,
                _ => final_key = part.to_string(),
            }
        }

        if !VALID_KEYS.contains(&final_key.as_str()) {
            return None;
        }

        Some(Self {
            key: final_key,
            ctrl,
            shift,
            alt,
        })
    }
}

/// Built-in keymap, used both as `Config::default()`'s keymaps and as the
/// fallback when `config.lua` doesn't define a `keymaps` table at all.
pub(super) fn default_keymaps() -> HashMap<KeyCombo, KeyAction> {
    let mut keymaps: HashMap<KeyCombo, KeyAction> = HashMap::new();
    keymaps.insert(KeyCombo::from_key("Space").unwrap(), KeyAction::TogglePlay);
    keymaps.insert(KeyCombo::from_key("N").unwrap(), KeyAction::NextTrack);
    keymaps.insert(KeyCombo::from_key("P").unwrap(), KeyAction::PrevTrack);
    keymaps.insert(KeyCombo::from_key("UpArrow").unwrap(), KeyAction::VolumeUp);
    keymaps.insert(
        KeyCombo::from_key("DownArrow").unwrap(),
        KeyAction::VolumeDown,
    );
    keymaps
}

/// Extract the `keymaps` table from Lua globals into a `HashMap`. Falls
/// back to `default_keymaps()` entirely if the table is missing. Individual
/// bad entries (unknown action name, unparseable combo, or a combo already
/// bound) are logged and skipped rather than failing the whole table.
pub(super) fn load_keymaps(globals: &mlua::Table) -> HashMap<KeyCombo, KeyAction> {
    let keymaps_table: mlua::Table = match globals.get("keymaps") {
        Ok(k) => k,
        Err(_) => return default_keymaps(),
    };

    let mut keymaps: HashMap<KeyCombo, KeyAction> = HashMap::new();

    for pair in keymaps_table.pairs::<String, String>() {
        let Ok((key, value)) = pair else { continue };

        let Some(action) = parse_action(&key) else {
            eprintln!("Error: {key} is not a valid keymap action.");
            continue;
        };

        let Some(combo) = KeyCombo::from_key(&value) else {
            eprintln!("Error: {value} is not a valid keymap combo.");
            continue;
        };

        if keymaps.contains_key(&combo) {
            eprintln!("Error: {key} appears more than once in keymaps.");
            continue;
        }

        keymaps.insert(combo, action);
    }

    keymaps
}

/// Maps a `config.lua` action name (e.g. `"toggle_play"`) to its
/// `KeyAction`. Returns `None` for unrecognized names.
fn parse_action(s: &str) -> Option<KeyAction> {
    match s {
        "toggle_play" => Some(KeyAction::TogglePlay),
        "next_track" => Some(KeyAction::NextTrack),
        "prev_track" => Some(KeyAction::PrevTrack),
        "volume_up" => Some(KeyAction::VolumeUp),
        "volume_down" => Some(KeyAction::VolumeDown),
        "seek_forward" => Some(KeyAction::SeekForward),
        "seek_backward" => Some(KeyAction::SeekBackward),
        "open_library" => Some(KeyAction::OpenLibrary),
        "open_playlists" => Some(KeyAction::OpenPlaylists),
        _ => None,
    }
}

pub const VALID_KEYS: &[&str] = &[
    "Backspace",
    "Tab",
    "Return",
    "Escape",
    "Backtab",
    "Delete",
    "AltGr",
    "CapsLock",
    "ShiftR",
    "ControlR",
    "Meta",
    "MetaR",
    "Space",
    "UpArrow",
    "DownArrow",
    "LeftArrow",
    "RightArrow",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "ScrollLock",
    "Pause",
    "SysReq",
    "Stop",
    "Menu",
    "Back",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "Circumflex",
    "Exclamation",
    "DoubleQuote",
    "Hash",
    "Dollar",
    "Percent",
    "Ampersand",
    "Underscore",
    "OpenParen",
    "CloseParen",
    "Asterisk",
    "Plus",
    "Pipe",
    "HyphenMinus",
    "OpenCurlyBracket",
    "CloseCurlyBracket",
    "Tilde",
    "Colon",
    "Semicolon",
    "LessThan",
    "Equals",
    "GreaterThan",
    "QuestionMark",
    "At",
    "Comma",
    "Period",
    "Slash",
    "BackQuote",
    "OpenBracket",
    "BackSlash",
    "CloseBracket",
    "Quote",
];
