use crate::curve::{Curve, CurveSegment};
use crate::nurbs::NurbsCurve;
use crate::point::Point2d;
use crate::primitives::{CircularArc, CubicBezier, LineSeg};

pub fn offset_curve(curve: &Curve, dist: f64) -> Curve {
    match curve {
        Curve::Line(l) => Curve::Line(l.offset_exact(dist)),
        Curve::Arc(a) => {
            let new_r = a.radius + dist;
            let r = if new_r <= 0.0 {
                new_r.abs().max(1e-12)
            } else {
                new_r
            };
            Curve::Arc(CircularArc::new(a.center, r, a.start_angle, a.end_angle))
        }
        Curve::Bezier(bz) => offset_bezier_approx(bz, dist),
        Curve::Nurbs(nc) => offset_nurbs(nc, dist),
        Curve::Poly(pc) => offset_polycurve(pc, dist),
        Curve::Ellipse(_) | Curve::Rational(_) => offset_by_sampling(curve, dist),
    }
}

fn offset_polycurve(pc: &crate::primitives::PolyCurve, dist: f64) -> Curve {
    use crate::primitives::PolyCurve;
    let n = pc.segments.len();
    if n == 0 {
        return Curve::Poly(Box::new(pc.clone()));
    }

    let mut offs: Vec<Curve> = pc.segments.iter().map(|s| offset_curve(s, dist)).collect();

    let (first_start, _) = seg_ends(&pc.segments[0]);
    let (_, last_end) = seg_ends(&pc.segments[n - 1]);
    let closed = (first_start.0 - last_end.0).hypot(first_start.1 - last_end.1) < 1e-9;

    let joints = if closed { n } else { n.saturating_sub(1) };
    for j in 0..joints {
        miter_join(&mut offs, j, (j + 1) % n);
    }
    Curve::Poly(Box::new(PolyCurve::new(offs)))
}

fn miter_join(offs: &mut [Curve], i: usize, k: usize) {
    let (a, b) = match (as_line_f64(&offs[i]), as_line_f64(&offs[k])) {
        (Some(a), Some(b)) => (a, b),
        _ => return,
    };
    let x =
        infinite_line_intersection(a, b).unwrap_or(((a.1.0 + b.0.0) * 0.5, (a.1.1 + b.0.1) * 0.5));
    set_line_p1(&mut offs[i], x);
    set_line_p0(&mut offs[k], x);
}

fn seg_ends(c: &Curve) -> ((f64, f64), (f64, f64)) {
    let (t0, t1) = c.domain();
    (c.evaluate_f64(t0), c.evaluate_f64(t1))
}

fn as_line_f64(c: &Curve) -> Option<((f64, f64), (f64, f64))> {
    match c {
        Curve::Line(l) => Some((l.p0.to_f64(), l.p1.to_f64())),
        _ => None,
    }
}

fn set_line_p0(c: &mut Curve, x: (f64, f64)) {
    if let Curve::Line(l) = c {
        l.p0 = Point2d::from_f64(x.0, x.1);
    }
}
fn set_line_p1(c: &mut Curve, x: (f64, f64)) {
    if let Curve::Line(l) = c {
        l.p1 = Point2d::from_f64(x.0, x.1);
    }
}

fn infinite_line_intersection(
    a: ((f64, f64), (f64, f64)),
    b: ((f64, f64), (f64, f64)),
) -> Option<(f64, f64)> {
    let imp =
        |((x0, y0), (x1, y1)): ((f64, f64), (f64, f64))| (y0 - y1, x1 - x0, x0 * y1 - x1 * y0);
    let (a1, b1, c1) = imp(a);
    let (a2, b2, c2) = imp(b);
    let det = a1 * b2 - a2 * b1;
    if det.abs() < 1e-12 {
        return None;
    }
    Some(((b1 * c2 - b2 * c1) / det, (a2 * c1 - a1 * c2) / det))
}

fn offset_nurbs(nc: &NurbsCurve, dist: f64) -> Curve {
    let m = nc.control.len();
    if m < 2 {
        return offset_by_sampling(&Curve::Nurbs(nc.clone()), dist);
    }
    let params: Vec<f64> = (0..m).map(|k| k as f64 / (m - 1) as f64).collect();
    let data: Vec<Point2d> = params
        .iter()
        .map(|&t| {
            let (px, py) = nc.evaluate_f64(t);
            let (tx, ty) = nc.tangent_f64(t);
            let len = (tx * tx + ty * ty).sqrt().max(1e-12);
            Point2d::from_f64(px + dist * (-ty / len), py + dist * (tx / len))
        })
        .collect();
    match interpolate_nurbs(&data, &nc.weights) {
        Some(fit) => Curve::Nurbs(fit),
        None => offset_by_sampling(&Curve::Nurbs(nc.clone()), dist),
    }
}

pub fn interpolate_nurbs(data: &[Point2d], weights: &[f64]) -> Option<NurbsCurve> {
    let m = data.len();
    if m < 2 || weights.len() != m {
        return None;
    }
    let params: Vec<f64> = (0..m).map(|k| k as f64 / (m - 1) as f64).collect();
    let mut qx: Vec<f64> = data.iter().map(|p| p.x).collect();
    let mut qy: Vec<f64> = data.iter().map(|p| p.y).collect();

    let mut mat = vec![vec![0.0; m]; m];
    for i in 0..m {
        let mut basis_ctrl = vec![Point2d::from_f64(0.0, 0.0); m];
        basis_ctrl[i] = Point2d::from_f64(1.0, 0.0);
        let probe = NurbsCurve::new(basis_ctrl, weights.to_vec());
        for (k, &t) in params.iter().enumerate() {
            mat[k][i] = probe.evaluate_f64(t).0;
        }
    }
    solve2(&mut mat, &mut qx, &mut qy).map(|()| {
        let control = qx
            .iter()
            .zip(&qy)
            .map(|(&x, &y)| Point2d::from_f64(x, y))
            .collect();
        NurbsCurve::new(control, weights.to_vec())
    })
}

pub fn refit_nurbs_subcurve(nc: &NurbsCurve, a: f64, b: f64) -> NurbsCurve {
    let m = nc.control.len().max(2);
    let data: Vec<Point2d> = (0..m)
        .map(|k| {
            let t = a + (b - a) * (k as f64 / (m - 1) as f64);
            let (x, y) = nc.evaluate_f64(t);
            Point2d::from_f64(x, y)
        })
        .collect();
    let weights = vec![1.0; m];
    interpolate_nurbs(&data, &weights).unwrap_or_else(|| nc.clone())
}

#[allow(clippy::needless_range_loop)]
fn solve2(a: &mut [Vec<f64>], b1: &mut [f64], b2: &mut [f64]) -> Option<()> {
    let n = a.len();
    for col in 0..n {
        let mut piv = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..n {
            if a[r][col].abs() > best {
                best = a[r][col].abs();
                piv = r;
            }
        }
        if best < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        b1.swap(col, piv);
        b2.swap(col, piv);
        // Eliminate below the pivot.
        for r in (col + 1)..n {
            let f = a[r][col] / a[col][col];
            if f == 0.0 {
                continue;
            }
            for c in col..n {
                a[r][c] -= f * a[col][c];
            }
            b1[r] -= f * b1[col];
            b2[r] -= f * b2[col];
        }
    }
    for col in (0..n).rev() {
        let mut s1 = b1[col];
        let mut s2 = b2[col];
        for c in (col + 1)..n {
            s1 -= a[col][c] * b1[c];
            s2 -= a[col][c] * b2[c];
        }
        b1[col] = s1 / a[col][col];
        b2[col] = s2 / a[col][col];
    }
    Some(())
}

fn offset_bezier_approx(bz: &CubicBezier, dist: f64) -> Curve {
    let ts = [0.0f64, 1.0 / 3.0, 2.0 / 3.0, 1.0];
    let mut offset_pts = [(0.0f64, 0.0f64); 4];

    for (i, &t) in ts.iter().enumerate() {
        let (px, py) = bz.evaluate_f64(t);
        let (tx, ty) = bz.tangent_f64(t);
        let len = (tx * tx + ty * ty).sqrt().max(1e-20);
        let (nx, ny) = (-ty / len, tx / len);
        offset_pts[i] = (px + dist * nx, py + dist * ny);
    }

    let p0 = Point2d::from_f64(offset_pts[0].0, offset_pts[0].1);
    let p3 = Point2d::from_f64(offset_pts[3].0, offset_pts[3].1);

    let (t0x, t0y) = bz.tangent_f64(0.0);
    let (t1x, t1y) = bz.tangent_f64(1.0);
    let chord = ((offset_pts[3].0 - offset_pts[0].0).powi(2)
        + (offset_pts[3].1 - offset_pts[0].1).powi(2))
    .sqrt();
    let scale = chord / 3.0;

    let p1 = Point2d::from_f64(
        offset_pts[0].0 + t0x * scale / (t0x * t0x + t0y * t0y).sqrt().max(1e-20),
        offset_pts[0].1 + t0y * scale / (t0x * t0x + t0y * t0y).sqrt().max(1e-20),
    );
    let p2 = Point2d::from_f64(
        offset_pts[3].0 - t1x * scale / (t1x * t1x + t1y * t1y).sqrt().max(1e-20),
        offset_pts[3].1 - t1y * scale / (t1x * t1x + t1y * t1y).sqrt().max(1e-20),
    );

    Curve::Bezier(CubicBezier::new(p0, p1, p2, p3))
}

fn offset_by_sampling(curve: &Curve, dist: f64) -> Curve {
    use crate::primitives::PolyCurve;

    let (t0, t1) = curve.domain();
    let steps = 16usize;
    let mut segs = Vec::new();
    let mut prev_pt: Option<(f64, f64)> = None;

    for i in 0..=steps {
        let t = t0 + (t1 - t0) * i as f64 / steps as f64;
        let (px, py) = curve.evaluate_f64(t);
        let (tx, ty) = curve.tangent_f64(t);
        let len = (tx * tx + ty * ty).sqrt().max(1e-20);
        let (nx, ny) = (-ty / len, tx / len);
        let op = (px + dist * nx, py + dist * ny);
        if let Some(prev) = prev_pt {
            segs.push(Curve::Line(LineSeg::from_endpoints(
                Point2d::from_f64(prev.0, prev.1),
                Point2d::from_f64(op.0, op.1),
            )));
        }
        prev_pt = Some(op);
    }
    Curve::Poly(Box::new(PolyCurve::new(segs)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::Point2d;
    use crate::primitives::LineSeg;

    fn pt(x: i64, y: i64) -> Point2d {
        Point2d::from_i64(x, y)
    }

    #[test]
    fn offset_horizontal_line() {
        let line = Curve::Line(LineSeg::from_endpoints(pt(0, 0), pt(4, 0)));
        let off = offset_curve(&line, 1.0);
        if let Curve::Line(l) = off {
            let y0 = l.p0.y;
            let y1 = l.p1.y;
            assert!((y0 - 1.0).abs() < 1e-5, "y0={}", y0);
            assert!((y1 - 1.0).abs() < 1e-5, "y1={}", y1);
        } else {
            panic!("Expected Line");
        }
    }

    #[test]
    fn offset_circle_increases_radius() {
        let arc = Curve::Arc(CircularArc::new(
            pt(0, 0),
            3.0,
            0.0,
            2.0 * std::f64::consts::PI,
        ));
        let off = offset_curve(&arc, 2.0);
        if let Curve::Arc(a) = off {
            assert!((a.radius - 5.0).abs() < 1e-9);
        } else {
            panic!("Expected Arc");
        }
    }

    #[test]
    fn offset_spline_stays_a_spline() {
        use crate::nurbs::NurbsCurve;
        let cvs = vec![pt(0, 0), pt(2, 4), pt(6, 4), pt(8, 0), pt(10, 4), pt(12, 0)];
        let spline = Curve::Nurbs(NurbsCurve::uniform(cvs.clone()));
        let dist = 1.0;
        let off = offset_curve(&spline, dist);

        let nc = match &off {
            Curve::Nurbs(nc) => nc,
            other => panic!("expected a spline offset, got {:?}", other),
        };
        assert_eq!(
            nc.control.len(),
            cvs.len(),
            "control-vertex count preserved"
        );

        let m = cvs.len();
        for k in 0..m {
            let t = k as f64 / (m - 1) as f64;
            let (px, py) = spline.evaluate_f64(t);
            let (ox, oy) = off.evaluate_f64(t);
            let d = ((ox - px).powi(2) + (oy - py).powi(2)).sqrt();
            assert!(
                (d - dist).abs() < 1e-6,
                "node {k}: offset distance {d}, want {dist}"
            );
        }
    }

    #[test]
    fn offset_square_polycurve_miters_corners() {
        use crate::primitives::PolyCurve;
        let segs = vec![
            Curve::Line(LineSeg::from_endpoints(pt(0, 0), pt(4, 0))),
            Curve::Line(LineSeg::from_endpoints(pt(4, 0), pt(4, 4))),
            Curve::Line(LineSeg::from_endpoints(pt(4, 4), pt(0, 4))),
            Curve::Line(LineSeg::from_endpoints(pt(0, 4), pt(0, 0))),
        ];
        let sq = Curve::Poly(Box::new(PolyCurve::new(segs)));
        let off = offset_curve(&sq, 1.0);
        let pc = match &off {
            Curve::Poly(pc) => pc,
            o => panic!("expected Poly, got {:?}", o),
        };
        assert_eq!(pc.segments.len(), 4, "still 4 sides — no jitter facets");

        let near = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).hypot(p.1 - q.1) < 1e-9;
        let (s0, e0) = seg_ends(&pc.segments[0]);
        assert!(
            near(s0, (1.0, 1.0)) && near(e0, (3.0, 1.0)),
            "bottom edge {s0:?}->{e0:?}"
        );
        for j in 0..4 {
            let (_, end) = seg_ends(&pc.segments[j]);
            let (start_next, _) = seg_ends(&pc.segments[(j + 1) % 4]);
            assert!(
                near(end, start_next),
                "corner {j} discontinuous: {end:?} vs {start_next:?}"
            );
        }
    }

    #[test]
    fn offset_circle_decreases_radius() {
        let arc = Curve::Arc(CircularArc::new(
            pt(0, 0),
            5.0,
            0.0,
            2.0 * std::f64::consts::PI,
        ));
        let off = offset_curve(&arc, -2.0);
        if let Curve::Arc(a) = off {
            assert!((a.radius - 3.0).abs() < 1e-9);
        } else {
            panic!("Expected Arc");
        }
    }
}
