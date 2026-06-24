//! Regenerates the executable / launcher icons from the app symbol SVG.
//!
//!     cargo run -p eiderflat_ui --example gen_app_icon
//!
//! Writes into `apps/eiderflat_app/assets/`:
//!   - `eiderflat.ico`  — embedded into the Windows .exe by the app build script
//!   - `eiderflat.png`  — 512px, source for the macOS .icns and Linux icon
//!   - `eiderflat.svg`  — scalable Linux icon (a copy of the symbol artwork)
//!
//! Re-run after changing `crates/eiderflat_ui/assets/logotype/symbol.svg`.

use std::path::Path;

fn main() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/eiderflat_app/assets");
    std::fs::create_dir_all(&assets).expect("create assets dir");

    let ico = eiderflat_ui::icons::app_icon_ico().expect("rasterize .ico");
    std::fs::write(assets.join("eiderflat.ico"), &ico).expect("write .ico");

    let png = eiderflat_ui::icons::app_icon_png(512).expect("rasterize .png");
    std::fs::write(assets.join("eiderflat.png"), &png).expect("write .png");

    let svg = include_str!("../assets/logotype/symbol.svg");
    std::fs::write(assets.join("eiderflat.svg"), svg).expect("write .svg");

    println!(
        "wrote eiderflat.ico ({} bytes), eiderflat.png ({} bytes), eiderflat.svg to {}",
        ico.len(),
        png.len(),
        assets.display()
    );
}
