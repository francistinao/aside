fn main() {
    // Declaring the app commands makes every window need an explicit permission for them.
    let manifest = tauri_build::AppManifest::new().commands(&[
        "session_start",
        "session_set_lines",
        "session_control",
        "session_estimate",
        "prompter_set_visible",
        "capture_status",
        "follow_stats",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
