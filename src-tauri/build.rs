fn main() {
    #[cfg(feature = "desktop-webview")]
    tauri_build::build()
}
