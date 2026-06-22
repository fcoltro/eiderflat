#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eiderflat_geometry::{CircularArc, Curve, LineSeg, Point2d, intersect};
use eiderflat_ui::{AppState, UiState, draw_ui, egui};

fn main() {
    // Capture any panic to the log file (the console may flash and close).
    std::panic::set_hook(Box::new(|info| {
        log_init();
        log(&format!("PANIC: {info}"));
    }));

    match std::env::args().nth(1).as_deref() {
        Some("demo") | Some("cli") | Some("--demo") => {
            run_demo();
        }
        _ => {
            log_init();
            if let Err(e) = run_gui() {
                log(&format!(
                    "GUI failed to start ({e}). Running the kernel demo instead."
                ));
                run_demo();
            }
        }
    }
}

fn log_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(std::env::temp_dir)
        .join("eiderflat_log.txt")
}

fn log_init() {
    let _ = std::fs::write(log_path(), "eiderFLAT log\n=============\n");
}

fn log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = writeln!(f, "{msg}");
    }
    eprintln!("{msg}");
}

struct EiderflatCad {
    app: AppState,
    ui: UiState,
}

impl eframe::App for EiderflatCad {
    // eframe 0.34 drives apps through `ui` (the old `ctx`-based `update` is
    // deprecated). All our chrome/canvas code builds its own panels on the
    // context, so we draw from `ui.ctx()` and leave the provided root `ui` empty.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The viewport is the egui painter with adaptive, zoom-aware tessellation:
        // smooth at any zoom, dependency-free, and exact where it matters (the
        // algebraic kernel), tessellated only for display.
        draw_ui(ui, &mut self.app, &mut self.ui);
    }
}

fn run_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        // Native OS window (standard title bar, controls, rounding, resize/snap).
        viewport: egui::ViewportBuilder::default()
            .with_title("eiderFLAT")
            .with_icon(std::sync::Arc::new(eiderflat_ui::icons::app_icon()))
            .with_min_inner_size([900.0, 560.0])
            .with_inner_size([1200.0, 800.0]),
        // 4× MSAA sharpens the edges of the tessellated UI/geometry (cheap on any
        // modern GPU); combined with pixel-snapped icon buttons it crisps the chrome.
        multisampling: 4,
        ..Default::default()
    };
    eframe::run_native(
        "eiderFLAT",
        options,
        Box::new(|_cc| {
            log("Window created. Using the adaptive-tessellation egui painter.");
            Ok(Box::new(EiderflatCad {
                app: AppState::new(1200.0, 800.0),
                ui: UiState::default(),
            }))
        }),
    )
}

fn run_demo() {
    println!("=== eiderFLAT — Geometry Kernel Demo ===\n");

    let line = Curve::Line(LineSeg::from_endpoints(
        Point2d::from_f64(-8.0, 7.25),
        Point2d::from_f64(8.0, -4.75),
    ));
    let circle = Curve::Arc(CircularArc::new(
        Point2d::from_f64(0.0, 0.0),
        5.0,
        0.0,
        std::f64::consts::TAU,
    ));

    println!("Curve 1 (line):   3x + 4y - 5 = 0");
    println!("Curve 2 (circle): x² + y² - 25 = 0\n");

    let hits = intersect(&line, &circle);
    println!("Found {} intersection point(s):\n", hits.len());
    for (i, h) in hits.iter().enumerate() {
        let (x, y) = h.point;
        println!("  Point {}: x = {:.10},  y = {:.10}", i + 1, x, y);
        let line_err = (3.0 * x + 4.0 * y - 5.0).abs();
        let circle_err = (x * x + y * y - 25.0).abs();
        println!("    Residual on line:   {:.2e}", line_err);
        println!("    Residual on circle: {:.2e}", circle_err);
    }

    println!("\nGeometry runs on f64 + tolerance (robust, NURBS-ready kernel).");
    println!("Run `eiderflat_app` (no args) to launch the interactive CAD application.");
}
