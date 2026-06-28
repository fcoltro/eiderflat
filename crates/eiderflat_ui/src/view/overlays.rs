use super::UiState;
use super::render::corner_glass_frame;
use crate::state::AppState;
use crate::tools::Tool;
use egui::{Color32, Stroke, pos2};
use eiderflat_geometry::{Curve, CurveSegment, Point2d, curvature_at, normal_at};

pub(super) fn curvature_comb(
    painter: &egui::Painter,
    app: &AppState,
    curve: &Curve,
    origin: egui::Pos2,
    scale: f64,
    samples: usize,
) {
    if let Curve::Poly(poly) = curve {
        for seg in &poly.segments {
            if seg.as_line().is_none() {
                curvature_comb(painter, app, seg, origin, scale, samples);
            }
        }
        return;
    }
    if curve.as_line().is_some() {
        return;
    }

    let to_screen = |wx: f64, wy: f64| super::render::world_to_screen_pos(app, origin, wx, wy);
    let (t0, t1) = curve.domain();
    let n = samples.max(2);
    let tooth = Stroke::new(1.0, Color32::from_rgb(190, 120, 255));
    let envelope = Stroke::new(1.5, Color32::from_rgb(150, 90, 230));
    let bb = curve.bounding_box();
    let (w, h) = (bb.max.x - bb.min.x, bb.max.y - bb.min.y);
    let diag = (w * w + h * h).sqrt();
    let min_tooth = diag * 1e-3;
    let max_tooth = (w.min(h) * 0.5).max(scale);

    let mut run: Vec<egui::Pos2> = Vec::new();
    let flush = |run: &mut Vec<egui::Pos2>| {
        if run.len() >= 2 {
            painter.add(egui::Shape::line(run.clone(), envelope));
        }
        run.clear();
    };
    for i in 0..=n {
        let t = t0 + (t1 - t0) * i as f64 / n as f64;
        let k = match curvature_at(curve, t) {
            Some(k) if k.is_finite() => k,
            _ => {
                flush(&mut run);
                continue;
            }
        };
        let (nx, ny) = normal_at(curve, t);
        let nlen = (nx * nx + ny * ny).sqrt();
        let mut d = -k * scale;
        if nlen < 1e-12 || d.abs() < min_tooth {
            flush(&mut run);
            continue;
        }
        d = d.clamp(-max_tooth, max_tooth);
        let (x, y) = curve.evaluate_f64(t);
        let base = to_screen(x, y);
        let tip = to_screen(x + nx / nlen * d, y + ny / nlen * d);
        painter.line_segment([base, tip], tooth);
        run.push(tip);
    }
    flush(&mut run);
}
pub(super) fn cursor_readout(ctx: &egui::Context, app: &AppState, origin: egui::Pos2) {
    if app.dyn_on {
        return;
    }
    let (cx, cy) = app.cursor_world;
    let text = match &app.tool {
        Tool::Move { base: Some(b), .. } | Tool::Copy { base: Some(b), .. } => {
            let (bx, by) = b.to_f64();
            let (dx, dy) = (cx - bx, cy - by);
            Some(format!(
                "Δ {:.2}, {:.2}   {:.2}",
                dx,
                dy,
                (dx * dx + dy * dy).sqrt()
            ))
        }
        Tool::Rotate { base: Some(b), .. } => {
            let (bx, by) = b.to_f64();
            let a = eiderflat_geometry::wrap_deg360((cy - by).atan2(cx - bx).to_degrees());
            Some(format!("{:.1}°", a))
        }
        Tool::Scale {
            base: Some(b),
            reference,
            ..
        } => {
            let (bx, by) = b.to_f64();
            let d = ((cx - bx).powi(2) + (cy - by).powi(2)).sqrt();
            match reference {
                Some(r) if *r > 1e-9 => Some(format!("×{:.3}", d / r)),
                _ => Some(format!("{:.2}", d)),
            }
        }
        _ => None,
    };
    let Some(text) = text else { return };

    let cur = app.view.world_to_screen(cx, cy);
    let pos = pos2(
        origin.x + cur.0 as f32 + 18.0,
        origin.y + cur.1 as f32 + 16.0,
    );
    egui::Area::new(egui::Id::new("cursor_readout"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(15, 19, 29, 200))
                .stroke(Stroke::new(1.0, crate::theme::OUTLINE))
                .corner_radius(crate::theme::tok::R_SM)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(text)
                            .monospace()
                            .size(12.0)
                            .color(crate::theme::ACCENT_BRIGHT),
                    );
                });
        });
}

fn hud_field(
    ui: &mut egui::Ui,
    id: egui::Id,
    buf: &mut String,
    width: f32,
    hint: &str,
    select_all: bool,
    grab_focus: bool,
) -> egui::Response {
    let out = egui::TextEdit::singleline(buf)
        .id(id)
        .desired_width(width)
        .hint_text(hint)
        .show(ui);
    if select_all {
        out.response.request_focus();
        let mut state = out.state;
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::select_all(&out.galley)));
        state.store(ui.ctx(), id);
    } else if grab_focus {
        out.response.request_focus();
    }
    out.response.response
}

fn hud_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .color(crate::theme::HUD_LABEL),
    );
}

fn cursor_hud_pos(app: &AppState, origin: egui::Pos2, dy: f32) -> egui::Pos2 {
    let (cx, cy) = app.cursor_world;
    let cur = app.view.world_to_screen(cx, cy);
    pos2(origin.x + cur.0 as f32 + 18.0, origin.y + cur.1 as f32 + dy)
}

fn cursor_hud(ctx: &egui::Context, id: &str, pos: egui::Pos2, add: impl FnOnce(&mut egui::Ui)) {
    egui::Area::new(egui::Id::new(id))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            corner_glass_frame().show(ui, |ui| {
                ui.horizontal(|ui| add(ui));
            });
        });
}

pub(super) fn dyn_transform_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    #[derive(Clone, Copy)]
    enum Kind {
        Translate,
        Rotate,
        Scale,
    }
    let info = match &app.tool {
        Tool::Move { base: Some(b), .. } | Tool::Copy { base: Some(b), .. } => {
            Some((Kind::Translate, b.to_f64(), None))
        }
        Tool::Rotate { base: Some(b), .. } => Some((Kind::Rotate, b.to_f64(), None)),
        Tool::Scale {
            base: Some(b),
            reference,
            ..
        } => Some((Kind::Scale, b.to_f64(), Some(*reference))),
        _ => None,
    };
    let (Some((kind, (bx, by), scale_ref)), true) = (info, app.dyn_on) else {
        ui_state.dyn_tf_active = false;
        return;
    };

    let (cx, cy) = app.cursor_world;
    let first_show = !ui_state.dyn_tf_active;
    ui_state.dyn_tf_active = true;

    let dx_id = egui::Id::new("dyn_tf_dx");
    let dy_id = egui::Id::new("dyn_tf_dy");
    let ang_id = egui::Id::new("dyn_tf_angle");
    let fac_id = egui::Id::new("dyn_tf_factor");

    let (dx, dy) = (cx - bx, cy - by);
    let cursor_ang = (cy - by).atan2(cx - bx);
    if !ctx.memory(|m| m.has_focus(dx_id)) {
        ui_state.dyn_tf_dx = format!("{dx:.2}");
    }
    if !ctx.memory(|m| m.has_focus(dy_id)) {
        ui_state.dyn_tf_dy = format!("{dy:.2}");
    }
    if !ctx.memory(|m| m.has_focus(ang_id)) {
        ui_state.dyn_tf_angle = format!("{:.1}", cursor_ang.to_degrees());
    }
    if !ctx.memory(|m| m.has_focus(fac_id)) {
        let live_factor = match scale_ref {
            Some(Some(r)) if r > 1e-9 => ((cx - bx).powi(2) + (cy - by).powi(2)).sqrt() / r,
            _ => 1.0,
        };
        ui_state.dyn_tf_factor = format!("{live_factor:.3}");
    }

    let nothing_focused = ctx.memory(|m| m.focused().is_none());
    let grab = first_show || nothing_focused;

    let pos = cursor_hud_pos(app, origin, -38.0);
    cursor_hud(ctx, "dyn_transform_hud", pos, |ui| match kind {
        Kind::Translate => {
            hud_label(ui, "ΔX");
            hud_field(
                ui,
                dx_id,
                &mut ui_state.dyn_tf_dx,
                56.0,
                "",
                first_show,
                grab,
            );
            hud_label(ui, "ΔY");
            hud_field(ui, dy_id, &mut ui_state.dyn_tf_dy, 56.0, "", false, false);
        }
        Kind::Rotate => {
            hud_field(
                ui,
                ang_id,
                &mut ui_state.dyn_tf_angle,
                56.0,
                "angle",
                first_show,
                grab,
            );
            hud_label(ui, "°");
        }
        Kind::Scale => {
            hud_label(ui, "×");
            hud_field(
                ui,
                fac_id,
                &mut ui_state.dyn_tf_factor,
                56.0,
                "factor",
                first_show,
                grab,
            );
        }
    });

    let mut commit = false;
    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        commit = true;
    }
    if !commit {
        return;
    }

    match kind {
        Kind::Translate => {
            let tdx = ui_state.dyn_tf_dx.trim().parse::<f64>().unwrap_or(dx);
            let tdy = ui_state.dyn_tf_dy.trim().parse::<f64>().unwrap_or(dy);
            app.place_tool_point(Point2d::from_f64(bx + tdx, by + tdy));
            ui_state.dyn_tf_active = false;
        }
        Kind::Rotate => {
            let Ok(mag) = ui_state.dyn_tf_angle.trim().parse::<f64>() else {
                return;
            };
            let dir = if cursor_ang >= 0.0 { 1.0 } else { -1.0 };
            let ang = dir * mag.abs().to_radians();
            app.place_tool_point(Point2d::from_f64(bx + ang.cos(), by + ang.sin()));
            ui_state.dyn_tf_active = false;
        }
        Kind::Scale => {
            let Ok(factor) = ui_state.dyn_tf_factor.trim().parse::<f64>() else {
                return;
            };
            if factor <= 1e-9 {
                return;
            }
            if let Tool::Scale { reference, .. } = &mut app.tool
                && reference.is_none()
            {
                *reference = Some(1.0);
            }
            let r1 = if let Tool::Scale {
                reference: Some(r), ..
            } = &app.tool
            {
                *r
            } else {
                1.0
            };
            app.place_tool_point(Point2d::from_f64(bx + factor * r1, by));
            ui_state.dyn_tf_active = false;
        }
    }
}

pub(super) fn dyn_line_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    let line_ref = if let Tool::Line { last: Some(p0) } = &app.tool {
        Some(p0.to_f64())
    } else {
        None
    };
    if let (true, Some((rx, ry))) = (app.dyn_on, line_ref) {
        let (cx, cy) = app.cursor_world;
        let live_len = ((cx - rx).powi(2) + (cy - ry).powi(2)).sqrt();
        let live_ang = eiderflat_geometry::wrap_deg360((cy - ry).atan2(cx - rx).to_degrees());

        let len_id = egui::Id::new("dyn_len");
        let ang_id = egui::Id::new("dyn_ang");
        if !ctx.memory(|m| m.has_focus(len_id)) {
            ui_state.dyn_length = format!("{:.2}", live_len);
        }
        if !ctx.memory(|m| m.has_focus(ang_id)) {
            ui_state.dyn_angle = format!("{:.1}", live_ang);
        }

        let first_show = !ui_state.dyn_active;
        let mut commit = false;
        let pos = cursor_hud_pos(app, origin, -38.0);
        cursor_hud(ctx, "dyn_input_hud", pos, |ui| {
            hud_label(ui, "L");
            let lr = hud_field(
                ui,
                len_id,
                &mut ui_state.dyn_length,
                58.0,
                "",
                false,
                first_show,
            );
            hud_label(ui, "∠");
            let ar = hud_field(ui, ang_id, &mut ui_state.dyn_angle, 48.0, "", false, false);
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && (lr.lost_focus() || ar.lost_focus())
            {
                commit = true;
            }
        });
        ui_state.dyn_active = true;
        if commit {
            let cmd = format!(
                "@{}<{}",
                ui_state.dyn_length.trim(),
                ui_state.dyn_angle.trim()
            );
            app.run_command(&cmd);
            ui_state.dyn_active = false;
        }
    } else {
        ui_state.dyn_active = false;
    }
}

pub(super) fn dyn_circle_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    let circle_center = if let Tool::Circle { center: Some(c) } = &app.tool {
        Some(c.to_f64())
    } else {
        None
    };
    if let (true, Some((cx, cy))) = (app.dyn_on, circle_center) {
        let rad_id = egui::Id::new("dyn_radius");
        let first_show = !ui_state.dyn_circle_active;
        if first_show {
            ui_state.dyn_radius.clear();
        }
        ui_state.dyn_circle_active = true;
        if ctx.input(|i| i.key_pressed(egui::Key::Enter))
            && let Ok(rad) = ui_state.dyn_radius.trim().parse::<f64>()
            && rad > 1e-9
        {
            app.place_tool_point(Point2d::from_f64(cx + rad, cy));
            ui_state.dyn_circle_active = false;
            return;
        }

        let pos = cursor_hud_pos(app, origin, -38.0);
        cursor_hud(ctx, "dyn_circle_hud", pos, |ui| {
            hud_label(ui, "R");
            let rr = hud_field(
                ui,
                rad_id,
                &mut ui_state.dyn_radius,
                58.0,
                "radius",
                false,
                false,
            );
            let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
            if first_show || nothing_focused {
                rr.request_focus();
            }
        });
    } else {
        ui_state.dyn_circle_active = false;
    }
}
pub(super) fn dyn_polygon_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    let sides = if let Tool::Polygon {
        center: None,
        sides,
    } = &app.tool
    {
        Some(*sides)
    } else {
        None
    };
    if let (true, Some(sides)) = (app.dyn_on, sides) {
        let sid = egui::Id::new("dyn_poly_sides");
        if !ctx.memory(|m| m.has_focus(sid)) {
            ui_state.dyn_poly_sides = sides.map(|n| n.to_string()).unwrap_or_default();
        }

        let first_show = !ui_state.dyn_poly_active;
        let pos = cursor_hud_pos(app, origin, -38.0);
        cursor_hud(ctx, "dyn_poly_hud", pos, |ui| {
            hud_label(ui, "Sides");
            let r = hud_field(
                ui,
                sid,
                &mut ui_state.dyn_poly_sides,
                40.0,
                "3+",
                false,
                false,
            );
            let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
            if first_show || nothing_focused {
                r.request_focus();
            }
        });
        ui_state.dyn_poly_active = true;
        let parsed = ui_state
            .dyn_poly_sides
            .trim()
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= 3);
        if parsed != sides {
            app.tool = Tool::Polygon {
                center: None,
                sides: parsed,
            };
        }
    } else {
        ui_state.dyn_poly_active = false;
    }
}

pub(super) fn dyn_rect_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    let rect_first = if let Tool::Rectangle { first: Some(f) } = &app.tool {
        Some(f.to_f64())
    } else {
        None
    };
    if let (true, Some((fx, fy))) = (app.dyn_on, rect_first) {
        let (crx, cry) = app.cursor_world;

        let field_id = egui::Id::new("dyn_rect_field");
        let first_show = !ui_state.dyn_rect_active;
        if first_show {
            ui_state.dyn_rect_width.clear();
            ui_state.dyn_rect_height.clear();
            ui_state.dyn_rect_stage_h = false;
        }
        ui_state.dyn_rect_active = true;
        let mut committed = false;
        let mut focus_field = first_show;
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if !ui_state.dyn_rect_stage_h {
                if let Ok(w) = ui_state.dyn_rect_width.trim().parse::<f64>()
                    && w.abs() > 1e-9
                {
                    ui_state.dyn_rect_stage_h = true;
                    focus_field = true;
                }
            } else if let Ok(h) = ui_state.dyn_rect_height.trim().parse::<f64>() {
                let w = ui_state.dyn_rect_width.trim().parse::<f64>().unwrap_or(0.0);
                if h.abs() > 1e-9 && w.abs() > 1e-9 {
                    let sx = if crx >= fx { 1.0 } else { -1.0 };
                    let sy = if cry >= fy { 1.0 } else { -1.0 };
                    app.place_tool_point(Point2d::from_f64(fx + w.abs() * sx, fy + h.abs() * sy));
                    ui_state.dyn_rect_active = false;
                    committed = true;
                }
            }
        }
        if committed {
            return;
        }

        let on_height = ui_state.dyn_rect_stage_h;
        let pos = cursor_hud_pos(app, origin, -38.0);
        cursor_hud(ctx, "dyn_rect_hud", pos, |ui| {
            let (label, buf, hint) = if on_height {
                ("H", &mut ui_state.dyn_rect_height, "height, Enter")
            } else {
                ("W", &mut ui_state.dyn_rect_width, "width, Enter")
            };
            hud_label(ui, label);
            let r = hud_field(ui, field_id, buf, 70.0, hint, false, false);
            let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
            if focus_field || nothing_focused {
                r.request_focus();
            }
        });
    } else {
        ui_state.dyn_rect_active = false;
    }
}
pub(super) fn dyn_ellipse_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    let stage = match &app.tool {
        Tool::Ellipse {
            center: Some(c),
            axis_end: None,
        } => Some((c.to_f64(), None)),
        Tool::Ellipse {
            center: Some(c),
            axis_end: Some(a),
        } => Some((c.to_f64(), Some(a.to_f64()))),
        _ => None,
    };
    if let (true, Some((center, axis_end))) = (app.dyn_on, stage) {
        let (crx, cry) = app.cursor_world;
        let first_show = !ui_state.dyn_ell_active;
        if first_show {
            ui_state.dyn_ell_major.clear();
            ui_state.dyn_ell_minor.clear();
        }
        ui_state.dyn_ell_active = true;
        let maj_id = egui::Id::new("dyn_ell_major");
        let min_id = egui::Id::new("dyn_ell_minor");
        let active_id = if axis_end.is_none() { maj_id } else { min_id };
        let tab = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab))
            | ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
        if tab {
            ctx.memory_mut(|m| m.request_focus(active_id));
        }

        let mut committed = false;
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            match axis_end {
                None => {
                    if let Ok(maj) = ui_state.dyn_ell_major.trim().parse::<f64>()
                        && maj.abs() > 1e-9
                    {
                        let dir = (crx - center.0, cry - center.1);
                        let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
                        let (ux, uy) = if len > 1e-9 {
                            (dir.0 / len, dir.1 / len)
                        } else {
                            (1.0, 0.0)
                        };
                        app.place_tool_point(Point2d::from_f64(
                            center.0 + maj * ux,
                            center.1 + maj * uy,
                        ));
                        ui_state.dyn_ell_active = false;
                        committed = true;
                    }
                }
                Some(a_end) => {
                    if let Ok(minr) = ui_state.dyn_ell_minor.trim().parse::<f64>()
                        && minr.abs() > 1e-9
                    {
                        let dir = (a_end.0 - center.0, a_end.1 - center.1);
                        let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt().max(1e-12);
                        let (px, py) = (-dir.1 / len, dir.0 / len);
                        app.place_tool_point(Point2d::from_f64(
                            center.0 + minr * px,
                            center.1 + minr * py,
                        ));
                        ui_state.dyn_ell_active = false;
                        committed = true;
                    }
                }
            }
        }
        if committed {
            return;
        }

        let pos = cursor_hud_pos(app, origin, -52.0);
        cursor_hud(ctx, "dyn_ell_hud", pos, |ui| {
            let (label, id, buf, hint) = if axis_end.is_none() {
                (
                    "A",
                    maj_id,
                    &mut ui_state.dyn_ell_major,
                    "major (aim with cursor)",
                )
            } else {
                ("B", min_id, &mut ui_state.dyn_ell_minor, "minor")
            };
            hud_label(ui, label);
            let r = hud_field(ui, id, buf, 54.0, hint, false, false);
            let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
            if first_show || nothing_focused {
                r.request_focus();
            }
        });
    } else {
        ui_state.dyn_ell_active = false;
    }
}

pub(super) fn dyn_offset_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    let dist = if let Tool::Offset { dist, .. } = &app.tool {
        Some(*dist)
    } else {
        None
    };
    if let (true, Some(dist)) = (app.dyn_on, dist) {
        let first_show = !ui_state.dyn_offset_active;
        if first_show {
            ui_state.dyn_offset_dist = super::render::trim_decimals(dist, 4);
        }
        let did = egui::Id::new("dyn_offset_dist");

        let pos = cursor_hud_pos(app, origin, -38.0);
        cursor_hud(ctx, "dyn_offset_hud", pos, |ui| {
            hud_label(ui, "Dist");
            let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
            hud_field(
                ui,
                did,
                &mut ui_state.dyn_offset_dist,
                58.0,
                "distance",
                first_show,
                !first_show && nothing_focused,
            );
        });
        ui_state.dyn_offset_active = true;
        if let Ok(d) = ui_state.dyn_offset_dist.trim().parse::<f64>()
            && d > 1e-9
            && let Tool::Offset { source, .. } = &app.tool
        {
            app.tool = Tool::Offset {
                dist: d,
                source: *source,
            };
        }
    } else {
        ui_state.dyn_offset_active = false;
    }
}
pub(super) fn dyn_corner_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    let info = match &app.tool {
        Tool::Fillet { radius, .. } => Some(("Radius", *radius)),
        Tool::Chamfer { dist, .. } => Some(("Dist", *dist)),
        Tool::CircleTtr { radius, .. } => Some(("Radius", *radius)),
        _ => None,
    };
    let (Some((label, value)), true) = (info, app.dyn_on) else {
        ui_state.dyn_corner_active = false;
        return;
    };

    let first_show = !ui_state.dyn_corner_active;
    if first_show {
        ui_state.dyn_corner_val = super::render::trim_decimals(value, 4);
    }
    let id = egui::Id::new("dyn_corner_val");
    let pos = cursor_hud_pos(app, origin, -38.0);
    cursor_hud(ctx, "dyn_corner_hud", pos, |ui| {
        hud_label(ui, label);
        let r = hud_field(
            ui,
            id,
            &mut ui_state.dyn_corner_val,
            58.0,
            "value, then pick lines",
            false,
            false,
        );
        let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
        if first_show || nothing_focused {
            r.request_focus();
        }
    });
    ui_state.dyn_corner_active = true;
    if let Ok(v) = ui_state.dyn_corner_val.trim().parse::<f64>()
        && v > 1e-9
    {
        match &app.tool {
            Tool::Fillet { first, .. } => {
                app.tool = Tool::Fillet {
                    radius: v,
                    first: *first,
                }
            }
            Tool::Chamfer { first, .. } => {
                app.tool = Tool::Chamfer {
                    dist: v,
                    first: *first,
                }
            }
            Tool::CircleTtr { first, .. } => {
                app.tool = Tool::CircleTtr {
                    radius: v,
                    first: *first,
                }
            }
            _ => {}
        }
    }
}

pub(super) fn dyn_text_hud(
    ctx: &egui::Context,
    app: &mut AppState,
    ui_state: &mut UiState,
    origin: egui::Pos2,
) {
    let anchor = if let Tool::Text {
        anchor: Some(a), ..
    } = &app.tool
    {
        Some(a.to_f64())
    } else {
        None
    };
    if let Some((ax, ay)) = anchor {
        let first_show = !ui_state.dyn_text_active;
        if first_show {
            ui_state.dyn_text_content.clear();
        }
        let tid = egui::Id::new("dyn_text_field");
        let sp = app.view.world_to_screen(ax, ay);
        let hud_pos = pos2(origin.x + sp.0 as f32, origin.y + sp.1 as f32 - 26.0);
        let mut commit = false;
        let mut cancel = false;
        egui::Area::new(egui::Id::new("dyn_text_hud"))
            .fixed_pos(hud_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let te = ui.add(
                            egui::TextEdit::singleline(&mut ui_state.dyn_text_content)
                                .id(tid)
                                .desired_width(180.0)
                                .hint_text("type text, Enter to place"),
                        );
                        ui.add_space(4.0);
                        super::chrome::font_combo(ui, "dyn_text_font", &mut app.text_font);
                        height_glyph(ui);
                        let mut size = if let Tool::Text { height, .. } = &app.tool {
                            *height
                        } else {
                            2.5
                        };
                        let dv = ui
                            .add(egui::DragValue::new(&mut size).speed(0.05).range(0.1..=1e6))
                            .on_hover_text("Text height");
                        if dv.changed()
                            && let Tool::Text { height, .. } = &mut app.tool
                        {
                            *height = size;
                        }
                        let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
                        if first_show || nothing_focused {
                            te.request_focus();
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            cancel = true;
                        } else if (te.lost_focus() || te.has_focus())
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            commit = true;
                        }
                    });
                });
            });
        ui_state.dyn_text_active = true;
        if commit {
            let content = std::mem::take(&mut ui_state.dyn_text_content);
            app.run_command(&content);
            ui_state.dyn_text_active = false;
        } else if cancel {
            app.tool = Tool::Select;
            ui_state.dyn_text_active = false;
        }
    } else {
        ui_state.dyn_text_active = false;
    }
}
fn height_glyph(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(11.0, 16.0), egui::Sense::hover());
    let x = rect.center().x;
    let (top, bot) = (rect.top() + 2.0, rect.bottom() - 2.0);
    let s = egui::Stroke::new(1.3, crate::theme::HUD_LABEL);
    let p = ui.painter();
    p.line_segment([pos2(x, top), pos2(x, bot)], s);
    for (y, dy) in [(top, 3.5_f32), (bot, -3.5_f32)] {
        p.line_segment([pos2(x, y), pos2(x - 3.0, y + dy)], s);
        p.line_segment([pos2(x, y), pos2(x + 3.0, y + dy)], s);
    }
}
