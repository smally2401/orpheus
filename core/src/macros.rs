/// Generates the boilerplate for wiring a `Theme` into a Slint UI's
/// setters. Each `item, property` pair expands to a call like
/// `ui.set_sidebar_bg(theme.bg.sidebar)`: i.e. `item` is the UI region
/// (`sidebar`, `now_playing_bar`, ...) and `property` is which theme
/// sub-struct it comes from (`bg`, `text_color`, `text_size`). Used in
/// `main.rs`'s `apply_theme` to avoid hand writing one setter call per
/// field.
#[macro_export]
macro_rules! theme_apply {
    ($ui:expr, $theme:expr, $( $item:ident, $property:ident );* $(;)? ) => {
        $(
            paste! {
                $ui.[<set_ $item _ $property>]($theme.$property.$item);
            }
        )*
    };
}

/// Declares which Slint `Key` variants Orpheus recognizes as valid keymap
/// targets, and generates two items from that list:
/// - `VALID_KEYS`: a `&[&str]` of the variant names, checked against by
///   `KeyCombo::from_key` when parsing a `config.lua` keybinding.
/// - `key_string_to_key_name`: converts a Slint key event's `SharedString`
///   back into one of those names.
#[macro_export]
macro_rules! define_keys {
    ($ ( $key:ident ),* $(,)?) => {
        pub const VALID_KEYS: &[&str] = &[
            $( stringify!($key) ),*
        ];

        pub fn key_string_to_key_name(s: SharedString) -> Option<String> {
            $(
                if s == SharedString::from(Key::$key) {
                    return Some(stringify!($key).to_string());
                }
            )*

            None
        }
    };
}
