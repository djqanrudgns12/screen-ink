const COMMANDS: &[&str] = &[
    "get_state",
    "update_setting",
    "preview_shortcut",
    "set_shortcut",
    "reset_shortcut_group",
    "undo",
    "set_hotkeys_enabled",
    "run",
    "preview_import",
    "apply_import",
    "reset_all",
    "overlay_ready",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
