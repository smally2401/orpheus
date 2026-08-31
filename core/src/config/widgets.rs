//! `WidgetSpec`: a small, composable set of UI primitives that a
//! `config.lua` script can describe to build custom widgets (e.g. a
//! standalone play/pause button, or a reordered row of transport
//! controls) out of a fixed set of building blocks. Parsed once from
//! Lua tables at config-load time (see `parse_widget_spec` below),
//! turning a `WidgetSpec` into actual Slint components happens on the
//! desktop side (`orpheus-desktop::config::widgets`).

use crate::player_bridge::PlayerCommand;
use std::collections::HashMap;

/// A player action a `WidgetSpec::Button` can trigger when clicked.
#[derive(Clone, Copy)]
pub enum WidgetAction {
    TogglePlay,
    Play,
    Pause,
    NextTrack,
    PrevTrack,
    ToggleShuffle,
    ToggleRepeat,
}

impl WidgetAction {
    fn from_str(s: &str) -> Option<Self> {
        use WidgetAction::*;

        Some(match s {
            "toggle_play" => TogglePlay,
            "play" => Play,
            "pause" => Pause,
            "next_track" => NextTrack,
            "prev_track" => PrevTrack,
            "toggle_shuffle" => ToggleShuffle,
            "toggle_repeat" => ToggleRepeat,
            _ => return None,
        })
    }

    #[must_use]
    pub fn to_command(&self) -> PlayerCommand {
        use crate::player_bridge::PlayerCommand::*;

        match self {
            Self::TogglePlay => TogglePlay,
            Self::Play => Play,
            Self::Pause => Pause,
            Self::NextTrack => NextTrack,
            Self::PrevTrack => PrevTrack,
            Self::ToggleShuffle => ToggleShuffle,
            Self::ToggleRepeat => ToggleRepeat,
        }
    }
}

/// A leaf node: either a clickable button or fixed empty space.
pub enum WidgetLeaf {
    Button {
        /// Path to an icon image, relative to the config directory
        /// (e.g. `"icons/play.png"`). `None` falls back to `icon_text`.
        icon: Option<String>,
        /// Fallback glyph/text shown when `icon` is `None` or fails to
        /// resolve.
        icon_text: Option<String>,
        action: WidgetAction,
        size: i32,
    },
    Spacer(i32),
}

/// A top-leveñ widget a script can define under `widgets.<name>` in
/// `config.lua`.
pub enum WidgetSpec {
    Row(Vec<WidgetLeaf>),
    Column(Vec<WidgetLeaf>),
}

/// Parses a single `WidgetLeaf` node from a Lua table. Malformed leaves
/// (missing/invalid `type`, unknown action name, wrong field type) return
/// an error describing what was wrong.
pub fn parse_widget_leaf(table: &mlua::Table) -> mlua::Result<WidgetLeaf> {
    let node_type: String = table
        .get("type")
        .map_err(|_| mlua::Error::runtime("widget leaf is missing required field 'type'"))?;

    match node_type.as_str() {
        "button" => {
            let icon: Option<String> = table.get("icon").ok();
            let icon_text: Option<String> = table.get("icon_text").ok();

            if icon.is_none() && icon_text.is_none() {
                return Err(mlua::Error::runtime(
                    "button widget needs an 'icon' or 'icon_text'",
                ));
            }

            let action_str: String = table.get("action").map_err(|_| {
                mlua::Error::runtime("button widget is missing required field 'action'")
            })?;

            let action = WidgetAction::from_str(&action_str).ok_or_else(|| {
                mlua::Error::runtime(format!("'{action_str}' is not a valid widget action"))
            })?;

            let size: i32 = table.get("size").unwrap_or(40);

            Ok(WidgetLeaf::Button {
                icon,
                icon_text,
                action,
                size,
            })
        }

        "spacer" => {
            let size: i32 = table.get("size").unwrap_or(8);
            Ok(WidgetLeaf::Spacer(size))
        }

        "row" | "column" => Err(mlua::Error::runtime(
            "widgets can only be nested one level deep, a row/column cannot hold another row or column"
        )),

        other => Err(mlua::Error::runtime(format!(
            "'{other}' is not a valid widget leaf type (expected 'button' or 'spacer')"
        ))),
    }
}

pub fn parse_widget_spec(table: &mlua::Table) -> mlua::Result<WidgetSpec> {
    let node_type: String = table
        .get("type")
        .map_err(|_| mlua::Error::runtime("widget is missing required field 'type'"))?;

    if node_type != "row" && node_type != "column" {
        return Err(mlua::Error::runtime(format!(
            "'{node_type}' is not a valid top-level widget type (expected 'row' or 'column')"
        )));
    }

    let children_table: mlua::Table = table
        .get("children")
        .map_err(|_| mlua::Error::runtime(format!("{node_type} widget is missing 'children'")))?;

    let mut children = Vec::new();
    for pair in children_table.sequence_values::<mlua::Table>() {
        children.push(parse_widget_leaf(&pair?)?);
    }

    Ok(if node_type == "row" {
        WidgetSpec::Row(children)
    } else {
        WidgetSpec::Column(children)
    })
}

/// Extracts the `widgets` table from Lua globals: a map of user-chosen
/// widget name to its `WidgetSpec` tree. A malformed individual widget
/// is logged and skipped, so one typo doesn't lose every custom widget
/// the user defined.
pub(super) fn load_widgets(globals: &mlua::Table) -> HashMap<String, WidgetSpec> {
    let widgets_table: mlua::Table = match globals.get("widgets") {
        Ok(w) => w,
        Err(_) => return HashMap::new(),
    };

    let mut widgets = HashMap::new();

    for pair in widgets_table.pairs::<String, mlua::Table>() {
        let Ok((name, table)) = pair else {
            continue;
        };

        match parse_widget_spec(&table) {
            Ok(spec) => {
                widgets.insert(name, spec);
            }
            Err(e) => {
                eprintln!("Error in widget '{name}': {e}");
            }
        }
    }

    widgets
}
