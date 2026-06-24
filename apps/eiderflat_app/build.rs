//! Embeds the application icon into the Windows `.exe` so it shows in Explorer,
//! the taskbar, and shortcuts. No-op on other platforms (Linux/macOS use a
//! `.desktop` file / `.app` bundle instead — see `assets/PACKAGING.md`).
//!
//! Regenerate `assets/eiderflat.ico` with:
//!   cargo run -p eiderflat_ui --example gen_app_icon
fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/eiderflat.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/eiderflat.ico");
        if let Err(e) = res.compile() {
            // Don't fail the build if the resource compiler (rc.exe / windres)
            // isn't available — the app just ships without an embedded icon.
            println!("cargo:warning=could not embed app icon: {e}");
        }
    }
}
