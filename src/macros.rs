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
