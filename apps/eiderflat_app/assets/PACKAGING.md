# Application icons

All icons derive from the app symbol
`crates/eiderflat_ui/assets/logotype/eiderFLAT_symbol.png` (a fixed-resolution
raster mark — there is no vector source). Regenerate the artifacts in this
folder after changing it:

```sh
cargo run -p eiderflat_ui --example gen_app_icon
```

This writes:

| File             | Used by                                                       |
|------------------|---------------------------------------------------------------|
| `eiderflat.ico`  | Windows `.exe` (embedded automatically by `build.rs`)         |
| `eiderflat.png`  | 512×512 source for the macOS `.icns` and Linux PNG icon       |

The **window/taskbar** icon is set at runtime in `main.rs` via
`eiderflat_ui::icons::app_icon()`, on every platform. The notes below are only
about the **file/launcher** icon shown by the OS file manager.

## Windows

Nothing to do — `build.rs` embeds `eiderflat.ico` into the executable, so
Explorer, the taskbar and shortcuts show it. (Requires a resource compiler:
the MSVC toolchain's `rc.exe`, or `windres` for the GNU toolchain. If neither is
found the build still succeeds, just without the embedded icon.)

## Linux

ELF binaries don't embed icons; the desktop environment reads a `.desktop`
file plus a themed icon. After `cargo build --release`:

```sh
install -Dm755 target/release/eiderflat        ~/.local/bin/eiderflat
install -Dm644 apps/eiderflat_app/assets/eiderflat.png \
    ~/.local/share/icons/hicolor/512x512/apps/eiderflat.png
install -Dm644 apps/eiderflat_app/assets/eiderflat.desktop \
    ~/.local/share/applications/eiderflat.desktop
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

(Use `/usr/local/bin` and `/usr/share/...` for a system-wide install.)

## macOS

The Finder icon lives in a `.app` bundle as an `.icns`. Build one from
`eiderflat.png` on a Mac:

```sh
mkdir eiderflat.iconset
for s in 16 32 64 128 256 512; do
    sips -z $s $s   eiderflat.png --out eiderflat.iconset/icon_${s}x${s}.png
    sips -z $((s*2)) $((s*2)) eiderflat.png --out eiderflat.iconset/icon_${s}x${s}@2x.png
done
iconutil -c icns eiderflat.iconset -o eiderflat.icns
```

Then place `eiderflat.icns` in `eiderFLAT.app/Contents/Resources/` and point
`CFBundleIconFile` at it in `Info.plist`. The easiest path is `cargo-bundle`,
which assembles the bundle and consumes the icon for you.
