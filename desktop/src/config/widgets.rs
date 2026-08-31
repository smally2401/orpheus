//! Converts a core `WidgetSpec`/`WidgetLeaf` tree into the Slint-facing
//! `WidgetLeaf`/`WidgetNodeKind` values generated from
//! `widgets(widget_node.slint)`.

use orpheus_core::player_bridge::PlayerCommand;
use orpheus_core::utils::expand_tilde;
use orpheus_core::config::widgets::WidgetAction;
use orpheus_core::config::widgets::WidgetLeaf as CoreLeaf;
use orpheus_core::config::widgets::WidgetSpec;
use slint::Image;
use slint::ModelRc;
use slint::SharedString;
use slint::VecModel;
use std::path::Path;
use crate::WidgetNodeKind;
use crate::WidgetLeaf;

/// Converts one core `WidgetLeaf` into the Slint generated `WidgetLeaf`
/// struct. Icon paths are resolved relative to `config_dir` and loader
/// eagerly. A missing or unreadable icon file falls back to an empty
/// image.
fn leaf_to_slint(leaf: &CoreLeaf, config_dir: &Path) -> WidgetLeaf {
    use orpheus_core::config::widgets::WidgetLeaf::*;
    match leaf {
        Button {
            icon,
            icon_text,
            action,
            size,
        } => WidgetLeaf {
            kind: WidgetNodeKind::Button,
            icon: icon
                .as_deref()
                .map(|p| load_icon(config_dir, p))
                .unwrap_or_default(),
            icon_text: icon_text.clone().unwrap_or_default().into(),
            action: action_name(*action).into(),
            size: *size,
        },

        Spacer(size) => WidgetLeaf {
            kind: WidgetNodeKind::Spacer,
            icon: Image::default(),
            icon_text: SharedString::default(),
            action: SharedString::default(),
            size: *size,
        },
    }
}

/// Loads an icon image from `config_dir.join(relative_path)`. Logs and
/// returns a blank image on failure.
fn load_icon(config_dir: &Path, relative_path: &str) -> Image {
    let path = config_dir.join(expand_tilde(relative_path));
    match Image::load_from_path(&path) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Could not load widget icon {}: {e}", path.display());
            Image::default()
        }
    }
}

/// Converts a `WidgetAction` back to the action name string Lua uses
/// for it, since `WidgetRow`/`WidgetColummn`'s `action-triggered`
/// callback reports the clicked leaf's action as a string (see
/// `widget_node.slint`).
fn action_name(action: WidgetAction) -> &'static str {
    use WidgetAction::*;
    match action {
        TogglePlay => "toggle_play",
        Play => "play",
        Pause => "pause",
        NextTrack => "next_track",
        PrevTrack => "prev_track",
        ToggleShuffle => "toggle_shuffle",
        ToggleRepeat => "toggle_repeat",
    }
}

/// Converts a full `WidgetSpec`  into a `(is_row, ModelRc<WidgetLeaf>)`
/// pair, ready to bind to a `WidgetRow`'s or `WidgetColumn`'s `leaves`
/// property depending on which one `is_row` says to instantiate.
pub(crate) fn widget_spec_to_slint(spec: &WidgetSpec, config_dir: &Path) -> (bool, ModelRc<WidgetLeaf>) {
    let (is_row, leaves) = match spec {
        WidgetSpec::Row(leaves) => (true, leaves),
        WidgetSpec::Column(leaves) => (false, leaves),
    };

    let slint_leaves: Vec<WidgetLeaf> = leaves.iter().map(|l| leaf_to_slint(l, config_dir)).collect();
    (is_row, ModelRc::new(VecModel::from(slint_leaves)))
}

pub(crate) fn handle_widget_action(action: &str, tx: &tokio::sync::mpsc::Sender<PlayerCommand>) {
    let Some(command) = parse_action_name(action) else {
        eprintln!("'{action}' is not a recognized widget action");
        return;
    };
    let _ = tx.try_send(command);
}

fn parse_action_name(s: &str) -> Option<PlayerCommand> {
    use WidgetAction::*;
    let action = match s {
        "toggle_play" => TogglePlay,
        "play" => Play,
        "pause" => Pause,
        "next_track" => NextTrack,
        "prev_track" => PrevTrack,
        "toggle_shuffle" => ToggleShuffle,
        "toggle_repeat" => ToggleRepeat,
        _ => return None,
    };
    Some(action.to_command())
}
