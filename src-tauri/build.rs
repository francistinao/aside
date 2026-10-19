fn main() {
    // Declaring the app commands makes every window need an explicit permission for them.
    let manifest = tauri_build::AppManifest::new().commands(&[
        "session_start",
        "session_set_lines",
        "session_control",
        "session_estimate",
        "vault_status",
        "vault_choose_folder",
        "vault_tree",
        "vault_reveal",
        "vault_search",
        "library_pin",
        "library_context_menu",
        "library_popup_menu",
        "vault_conflict_dialog",
        "platform_info",
        "library_flushed",
        "vault_read",
        "vault_write",
        "vault_create",
        "vault_create_folder",
        "vault_rename",
        "vault_trash",
        "settings_get",
        "settings_set",
        "shortcuts_set",
        "shortcuts_reset",
        "shortcuts_status",
        "prompter_resize",
        "capture_status",
        "follow_stats",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
