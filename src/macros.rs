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
