#[cfg(feature = "gui")]
pub fn launch() -> tauri::Result<()> {
    tauri::Builder::default()
        .run(tauri::generate_context!())
}
