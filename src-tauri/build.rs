fn main() {
    // Declaring the app commands makes every window need an explicit permission for them.
    let manifest = tauri_build::AppManifest::new().commands(&[
        "prompter_set_visible",
        "capture_status",
        "follow_stats",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
