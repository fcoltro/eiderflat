use eiderflat_document::EntityKind;
use eiderflat_geometry::{Curve, LineSeg, Point2d};
use eiderflat_ui::AppState;

fn line(x0: i64, y0: i64, x1: i64, y1: i64) -> EntityKind {
    EntityKind::Curve(Curve::Line(LineSeg::from_endpoints(
        Point2d::from_i64(x0, y0),
        Point2d::from_i64(x1, y1),
    )))
}

fn click(a: &mut AppState, wx: f64, wy: f64) {
    let (sx, sy) = a.view.world_to_screen(wx, wy);
    a.canvas_click(sx, sy);
}

fn app() -> AppState {
    let mut a = AppState::new(1200.0, 800.0);
    a.snap_on = false;
    a
}

#[test]
fn trim_tool_cuts_picked_span() {
    let mut a = app();
    a.add_entity(line(0, 0, 10, 0));
    a.add_entity(line(3, -1, 3, 1));
    a.add_entity(line(7, -1, 7, 1));
    let before = a.document.len();
    a.run_command("TRIM");
    click(&mut a, 5.0, 0.0);
    assert_eq!(
        a.document.len(),
        before + 1,
        "trim should split target into two"
    );
}

#[test]
fn trim_ignores_object_snap_when_picking() {
    let mut a = AppState::new(1200.0, 800.0);
    a.snap_on = true;
    a.add_entity(line(0, 0, 10, 0));
    a.add_entity(line(3, -1, 3, 1));
    a.add_entity(line(7, -1, 7, 1));
    let before = a.document.len();
    a.run_command("TRIM");
    let (sx, sy) = a.view.world_to_screen(3.1, 0.05);
    a.pointer_moved(sx, sy);
    assert!(
        a.active_snap.is_none(),
        "entity-picking tools must not object-snap"
    );
    click(&mut a, 5.0, 0.0);
    assert_eq!(
        a.document.len(),
        before + 1,
        "trim must still cut the picked span"
    );
}

#[test]
fn offset_tool_creates_parallel_curve() {
    let mut a = app();
    a.add_entity(line(0, 0, 10, 0));
    let before = a.document.len();
    a.run_command("OFFSET");
    a.run_command("2");
    click(&mut a, 5.0, 0.0);
    click(&mut a, 5.0, 4.0);
    assert_eq!(
        a.document.len(),
        before + 1,
        "offset should add one parallel curve"
    );
}

#[test]
fn fillet_tool_adds_arc() {
    let mut a = app();
    a.add_entity(line(10, 0, 0, 0));
    a.add_entity(line(0, 0, 0, 10));
    let before = a.document.len();
    a.run_command("FILLET");
    a.run_command("2");
    click(&mut a, 5.0, 0.0);
    click(&mut a, 0.0, 5.0);
    assert_eq!(
        a.document.len(),
        before + 1,
        "fillet adds one arc (lines trimmed in place)"
    );
    assert!(
        a.document
            .iter()
            .any(|e| matches!(&e.kind, EntityKind::Curve(Curve::Arc(_)))),
        "a fillet arc should exist"
    );
}

#[test]
fn blend_tool_connects_two_entities_with_a_spline() {
    let mut a = app();
    a.add_entity(line(0, 0, 2, 0)); // ends at (2,0)
    a.add_entity(line(5, 0, 7, 0)); // starts at (5,0)
    let before = a.document.len();
    a.run_command("BLEND"); // default G1
    click(&mut a, 1.0, 0.0); // pick first line
    click(&mut a, 6.0, 0.0); // pick second line — stages a preview, doesn't commit yet
    assert_eq!(
        a.document.len(),
        before,
        "picking both entities only stages a preview; nothing is added until confirmed"
    );
    a.confirm_pending_blend();
    assert_eq!(
        a.document.len(),
        before + 1,
        "blend adds one connecting curve, leaving the sources in place"
    );
    assert!(
        a.document
            .iter()
            .any(|e| matches!(&e.kind, EntityKind::Curve(Curve::Bezier(_)))),
        "a G1 blend is a cubic Bézier"
    );
}

#[test]
fn blend_command_selects_g2_continuity() {
    let mut a = app();
    a.add_entity(line(0, 0, 2, 0));
    a.add_entity(line(5, 2, 7, 2));
    let before = a.document.len();
    a.run_command("BLEND G2");
    click(&mut a, 1.0, 0.0);
    click(&mut a, 6.0, 2.0);
    a.confirm_pending_blend();
    assert_eq!(a.document.len(), before + 1);
    assert!(
        a.document
            .iter()
            .any(|e| matches!(&e.kind, EntityKind::Curve(Curve::Rational(_)))),
        "a G2 blend is a quintic rational Bézier"
    );
}

#[test]
fn blend_pending_pick_can_be_cancelled_without_committing() {
    let mut a = app();
    a.add_entity(line(0, 0, 2, 0));
    a.add_entity(line(5, 0, 7, 0));
    let before = a.document.len();
    a.run_command("BLEND");
    click(&mut a, 1.0, 0.0);
    click(&mut a, 6.0, 0.0);
    a.cancel_pending_blend();
    assert_eq!(
        a.document.len(),
        before,
        "cancelling a pending blend must not add anything"
    );
    assert!(
        matches!(
            a.tool,
            eiderflat_ui::tools::Tool::Blend {
                first: None,
                second: None,
                ..
            }
        ),
        "cancel returns to picking a fresh first entity, tool={:?}",
        a.tool
    );
}

#[test]
fn blend_pending_pick_absorbs_further_clicks() {
    let mut a = app();
    a.add_entity(line(0, 0, 2, 0));
    a.add_entity(line(5, 0, 7, 0));
    a.add_entity(line(0, 3, 2, 3));
    let before = a.document.len();
    a.run_command("BLEND");
    click(&mut a, 1.0, 0.0); // first
    click(&mut a, 6.0, 0.0); // second — now pending confirmation
    click(&mut a, 1.0, 3.0); // a third click must not re-pick or commit anything
    assert_eq!(
        a.document.len(),
        before,
        "clicks while the confirm popup is pending must be absorbed, not picked"
    );
    assert!(
        matches!(
            a.tool,
            eiderflat_ui::tools::Tool::Blend {
                second: Some(_),
                ..
            }
        ),
        "tool stays pending until confirmed or cancelled, tool={:?}",
        a.tool
    );
}

#[test]
fn fillet_triangle_caps_radius_across_shared_edges() {
    use eiderflat_ui::state::CornerKind;
    use std::collections::HashMap;

    let mut a = app();
    let i1 = a.add_entity(line(0, 0, 10, 0));
    let i2 = a.add_entity(line(10, 0, 0, 10));
    let i3 = a.add_entity(line(0, 10, 0, 0));
    a.selection = vec![i1, i2, i3];

    let corners = a.detect_corners();
    assert_eq!(corners.len(), 3, "triangle has three corners");

    let cap = a.corner_group_cap(&corners[0], CornerKind::Fillet);

    let mut budget: HashMap<_, (f64, f64)> = HashMap::new();
    for c in &corners {
        let t = cap / (c.interior_angle() * 0.5).tan();
        for (id, len) in [(c.a, c.len_a), (c.b, c.len_b)] {
            let e = budget.entry(id).or_insert((0.0, f64::INFINITY));
            e.0 += t;
            e.1 = e.1.min(len);
        }
    }
    for (sum_t, len) in budget.values() {
        assert!(
            *sum_t <= *len + 1e-6,
            "fillet tangents {sum_t:.3} overrun a shared edge of length {len:.3}"
        );
    }

    a.begin_corner_action(corners[0]);
    a.set_corner_size(1e6);
    a.apply_corner_action();
    let arcs = a
        .document
        .iter()
        .filter(|e| matches!(&e.kind, EntityKind::Curve(Curve::Arc(_))))
        .count();
    assert_eq!(arcs, 3, "one fillet arc per corner");
    for e in a.document.iter() {
        if let EntityKind::Curve(Curve::Line(l)) = &e.kind {
            for p in [l.p0.to_f64(), l.p1.to_f64()] {
                assert!(
                    p.0 > -0.5 && p.0 < 10.5 && p.1 > -0.5 && p.1 < 10.5,
                    "line endpoint {p:?} escaped the triangle"
                );
            }
        }
    }
}

#[test]
fn fillet_triangle_arcs_connect_to_trimmed_lines() {
    let mut a = app();
    let i1 = a.add_entity(line(0, 0, 40, 2));
    let i2 = a.add_entity(line(40, 2, 15, 25));
    let i3 = a.add_entity(line(15, 25, 0, 0));
    a.selection = vec![i1, i2, i3];

    let corners = a.detect_corners();
    assert_eq!(corners.len(), 3);
    a.begin_corner_action(corners[0]);
    a.set_corner_size(1e6);
    a.apply_corner_action();

    let mut line_pts: Vec<(f64, f64)> = Vec::new();
    let mut arc_pts: Vec<(f64, f64)> = Vec::new();
    let mut n_arcs = 0;
    for e in a.document.iter() {
        match &e.kind {
            EntityKind::Curve(Curve::Line(l)) => {
                line_pts.push(l.p0.to_f64());
                line_pts.push(l.p1.to_f64());
            }
            EntityKind::Curve(Curve::Arc(arc)) => {
                n_arcs += 1;
                arc_pts.push(arc.start_point());
                arc_pts.push(arc.end_point());
            }
            _ => {}
        }
    }
    assert_eq!(n_arcs, 3, "one fillet arc per corner");
    for ap in &arc_pts {
        let connected = line_pts
            .iter()
            .any(|lp| (lp.0 - ap.0).hypot(lp.1 - ap.1) < 1e-6);
        assert!(connected, "fillet arc endpoint {ap:?} is disconnected");
    }
    for e in a.document.iter() {
        if let EntityKind::Curve(Curve::Line(l)) = &e.kind {
            let (p0, p1) = (l.p0.to_f64(), l.p1.to_f64());
            assert!(
                (p1.0 - p0.0).hypot(p1.1 - p0.1) > 1e-6,
                "a trimmed side collapsed"
            );
        }
    }
}

#[test]
fn grip_drag_tracks_the_lines_original_axis() {
    let mut a = app();
    a.track_on = true;
    let l1 = a.add_entity(line(0, 0, 10, 0));
    a.add_entity(line(3, 5, 3, 12));

    a.selection = vec![l1];
    let grip = a
        .selection_grips()
        .into_iter()
        .find(|(id, g)| *id == l1 && (g.world.to_f64().0 - 10.0).abs() < 1e-6)
        .map(|(_, g)| g)
        .expect("line should expose an endpoint grip at (10,0)");
    a.begin_grip_drag(l1, grip);

    let tol = a.view.pixel_world_size() * 10.0;
    let (sx, sy) = a.view.world_to_screen(14.0, tol * 0.5);
    a.pointer_moved(sx, sy);

    let labels: Vec<&str> = a
        .interaction
        .active_guides
        .iter()
        .map(|g| g.kind.label())
        .collect();
    assert_eq!(labels, vec!["Extension"], "only the edited line's axis");
    assert!(
        a.cursor_world.1.abs() < 1e-6,
        "cursor should snap back onto y=0, got {:?}",
        a.cursor_world
    );
    assert!((a.cursor_world.0 - 14.0).abs() < 1e-6);
}

#[test]
fn rotate_tool_turns_selection() {
    let mut a = app();
    let id = a.add_entity(line(1, 0, 2, 0));
    a.selection = vec![id];
    a.run_command("ROTATE");
    click(&mut a, 0.0, 0.0);
    click(&mut a, 0.0, 1.0);
    if let Some(Curve::Line(l)) = a.document.get(id).unwrap().as_curve() {
        assert!(
            l.p0.x.abs() < 1e-4 && (l.p0.y - 1.0).abs() < 1e-4,
            "(1,0) → (0,1), got {:?}",
            l.p0.to_f64()
        );
    } else {
        panic!("expected a line")
    }
}

#[test]
fn mirror_tool_reflects_selection() {
    let mut a = app();
    let id = a.add_entity(line(1, 2, 3, 4));
    a.selection = vec![id];
    a.run_command("MIRROR");
    click(&mut a, 0.0, 0.0);
    click(&mut a, 1.0, 0.0);
    if let Some(Curve::Line(l)) = a.document.get(id).unwrap().as_curve() {
        let (x, y) = l.p0.to_f64();
        assert!(
            (x - 1.0).abs() < 1e-4 && (y + 2.0).abs() < 1e-4,
            "(1,2) → (1,-2), got ({x},{y})"
        );
    } else {
        panic!("expected a line")
    }
}
