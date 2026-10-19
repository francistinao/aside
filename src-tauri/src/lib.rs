pub mod capture;
pub mod error;
pub mod follow;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
