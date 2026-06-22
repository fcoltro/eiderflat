use crate::curve::{Curve, CurveSegment};
use crate::point::{BoundingBox, Point2d};

#[derive(Clone, Debug, PartialEq)]
pub struct RationalBezier {
    pub points: Vec<Point2d>,
    pub weights: Vec<f64>,
}

impl RationalBezier {
    pub fn new(points: Vec<Point2d>, weights: Vec<f64>) -> Self {
        assert_eq!(
            points.len(),
            weights.len(),
            "points and weights must match in length"
        );
        assert!(
            points.len() >= 2,
            "a Bézier needs at least two control points"
        );
        assert!(
            weights.iter().all(|&w| w > 0.0),
            "weights must be strictly positive"
        );
        RationalBezier { points, weights }
    }

    pub fn polynomial(points: Vec<Point2d>) -> Self {
        let weights = vec![1.0; points.len()];
        RationalBezier::new(points, weights)
    }

    pub fn degree(&self) -> usize {
        self.points.len() - 1
    }

    fn homogeneous(&self) -> Vec<[f64; 3]> {
        self.points
            .iter()
            .zip(&self.weights)
            .map(|(p, &w)| [w * p.x, w * p.y, w])
            .collect()
    }

    fn from_homogeneous(h: &[[f64; 3]]) -> RationalBezier {
        let points = h
            .iter()
            .map(|c| Point2d::new(c[0] / c[2], c[1] / c[2]))
            .collect();
        let weights = h.iter().map(|c| c[2]).collect();
        RationalBezier { points, weights }
    }

    pub fn evaluate(&self, t: f64) -> Point2d {
        let [x, y, w] = de_casteljau(&self.homogeneous(), t);
        Point2d::new(x / w, y / w)
    }

    pub fn tangent(&self, t: f64) -> (f64, f64) {
        let h = self.homogeneous();
        let [hx, hy, hw] = de_casteljau(&h, t);
        let n = self.degree() as f64;
        let d: Vec<[f64; 3]> = h
            .windows(2)
            .map(|w| {
                [
                    n * (w[1][0] - w[0][0]),
                    n * (w[1][1] - w[0][1]),
                    n * (w[1][2] - w[0][2]),
                ]
            })
            .collect();
        let [dx, dy, dw] = if d.is_empty() {
            [0.0, 0.0, 0.0]
        } else {
            de_casteljau(&d, t)
        };
        let inv = 1.0 / (hw * hw);
        ((dx * hw - hx * dw) * inv, (dy * hw - hy * dw) * inv)
    }

    pub fn split(&self, t: f64) -> (RationalBezier, RationalBezier) {
        let mut level = self.homogeneous();
        let mut left = vec![level[0]];
        let mut right = vec![*level.last().unwrap()];
        while level.len() > 1 {
            let next: Vec<[f64; 3]> = level
                .windows(2)
                .map(|w| {
                    [
                        (1.0 - t) * w[0][0] + t * w[1][0],
                        (1.0 - t) * w[0][1] + t * w[1][1],
                        (1.0 - t) * w[0][2] + t * w[1][2],
                    ]
                })
                .collect();
            left.push(next[0]);
            right.push(*next.last().unwrap());
            level = next;
        }
        right.reverse();
        (
            RationalBezier::from_homogeneous(&left),
            RationalBezier::from_homogeneous(&right),
        )
    }

    pub fn reverse(&self) -> RationalBezier {
        let mut points = self.points.clone();
        let mut weights = self.weights.clone();
        points.reverse();
        weights.reverse();
        RationalBezier { points, weights }
    }

    pub fn bounding_box(&self) -> BoundingBox {
        let mut xmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymin = f64::INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        for p in &self.points {
            xmin = xmin.min(p.x);
            xmax = xmax.max(p.x);
            ymin = ymin.min(p.y);
            ymax = ymax.max(p.y);
        }
        BoundingBox::from_corners(xmin, ymin, xmax, ymax)
    }

    pub fn arc_length(&self) -> f64 {
        const NODES: [f64; 5] = [0.046910077, 0.230765346, 0.5, 0.769234654, 0.953089923];
        const WEIGHTS: [f64; 5] = [
            0.118463442,
            0.239314335,
            0.284444444,
            0.239314335,
            0.118463442,
        ];
        NODES.iter().zip(WEIGHTS.iter()).fold(0.0, |acc, (&t, &w)| {
            let (dx, dy) = self.tangent(t);
            acc + w * (dx * dx + dy * dy).sqrt()
        })
    }

    pub fn to_polyline(&self, tol: f64) -> Vec<Point2d> {
        let mut out = vec![self.evaluate(0.0)];
        self.flatten_into(0.0, 1.0, tol, 0, &mut out);
        out
    }

    fn flatten_into(&self, t0: f64, t1: f64, tol: f64, depth: u32, out: &mut Vec<Point2d>) {
        let a = self.evaluate(t0);
        let b = self.evaluate(t1);
        let tm = 0.5 * (t0 + t1);
        let m = self.evaluate(tm);
        let cmx = 0.5 * (a.x + b.x);
        let cmy = 0.5 * (a.y + b.y);
        let dev = ((m.x - cmx).powi(2) + (m.y - cmy).powi(2)).sqrt();
        if dev <= tol || depth >= 24 {
            out.push(b);
        } else {
            self.flatten_into(t0, tm, tol, depth + 1, out);
            self.flatten_into(tm, t1, tol, depth + 1, out);
        }
    }
}

impl CurveSegment for RationalBezier {
    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }
    fn evaluate_f64(&self, t: f64) -> (f64, f64) {
        let p = self.evaluate(t);
        (p.x, p.y)
    }
    fn bounding_box(&self) -> BoundingBox {
        self.bounding_box()
    }
    fn tangent_f64(&self, t: f64) -> (f64, f64) {
        self.tangent(t)
    }
    fn arc_length(&self) -> f64 {
        self.arc_length()
    }
}

fn de_casteljau(control: &[[f64; 3]], t: f64) -> [f64; 3] {
    let mut h = control.to_vec();
    let n = h.len();
    for r in 1..n {
        for i in 0..n - r {
            let (a, b) = (h[i], h[i + 1]);
            h[i] = [
                (1.0 - t) * a[0] + t * b[0],
                (1.0 - t) * a[1] + t * b[1],
                (1.0 - t) * a[2] + t * b[2],
            ];
        }
    }
    h[0]
}

pub fn lower(curve: &Curve) -> Vec<RationalBezier> {
    match curve {
        Curve::Line(l) => vec![RationalBezier::polynomial(vec![l.p0, l.p1])],
        Curve::Bezier(b) => vec![RationalBezier::polynomial(vec![b.p0, b.p1, b.p2, b.p3])],
        Curve::Arc(a) => {
            let (cx, cy, r) = (a.center.x, a.center.y, a.radius);
            unit_arc_segments(a.start_angle, a.end_angle)
                .into_iter()
                .map(|(cps, w)| {
                    let map = |p: [f64; 2]| Point2d::new(cx + r * p[0], cy + r * p[1]);
                    RationalBezier::new(
                        vec![map(cps[0]), map(cps[1]), map(cps[2])],
                        vec![1.0, w, 1.0],
                    )
                })
                .collect()
        }
        Curve::Ellipse(e) => {
            let (sin_phi, cos_phi) = e.rotation.sin_cos();
            let (cx, cy, sa, sb) = (e.center.x, e.center.y, e.semi_major, e.semi_minor);
            let map = |p: [f64; 2]| {
                let (u, v) = (sa * p[0], sb * p[1]);
                Point2d::new(
                    cx + u * cos_phi - v * sin_phi,
                    cy + u * sin_phi + v * cos_phi,
                )
            };
            unit_arc_segments(e.start_angle, e.end_angle)
                .into_iter()
                .map(|(cps, w)| {
                    RationalBezier::new(
                        vec![map(cps[0]), map(cps[1]), map(cps[2])],
                        vec![1.0, w, 1.0],
                    )
                })
                .collect()
        }
        Curve::Poly(pc) => pc.segments.iter().flat_map(lower).collect(),
        Curve::Rational(rb) => vec![rb.clone()],
        Curve::Nurbs(nc) => nc.segments(),
    }
}

pub fn tessellate_curve(curve: &Curve, tol: f64) -> Vec<Point2d> {
    let mut out: Vec<Point2d> = Vec::new();
    for (i, seg) in lower(curve).iter().enumerate() {
        let poly = seg.to_polyline(tol);
        if i == 0 {
            out.extend(poly);
        } else {
            out.extend(poly.into_iter().skip(1));
        }
    }
    out
}

fn unit_arc_segments(a0: f64, a1: f64) -> Vec<([[f64; 2]; 3], f64)> {
    let sweep = a1 - a0;
    let n = ((sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = sweep / n as f64;
    (0..n)
        .map(|i| {
            let b0 = a0 + step * i as f64;
            let b1 = b0 + step;
            let half = 0.5 * (b1 - b0);
            let mid = 0.5 * (b0 + b1);
            let w = half.cos();
            let p0 = [b0.cos(), b0.sin()];
            let p2 = [b1.cos(), b1.sin()];
            let p1 = [mid.cos() / w, mid.sin() / w];
            ([p0, p1, p2], w)
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct NurbsCurve {
    pub control: Vec<Point2d>,
    pub weights: Vec<f64>,
}

impl NurbsCurve {
    pub fn new(control: Vec<Point2d>, weights: Vec<f64>) -> Self {
        assert_eq!(
            control.len(),
            weights.len(),
            "control and weights must match in length"
        );
        assert!(
            control.len() >= 2,
            "a spline needs at least two control vertices"
        );
        assert!(
            weights.iter().all(|&w| w > 0.0),
            "weights must be strictly positive"
        );
        NurbsCurve { control, weights }
    }

    pub fn uniform(control: Vec<Point2d>) -> Self {
        let weights = vec![1.0; control.len()];
        NurbsCurve::new(control, weights)
    }

    pub fn segments(&self) -> Vec<RationalBezier> {
        cv_spline_segments_weighted(&self.control, &self.weights)
    }
}

impl CurveSegment for NurbsCurve {
    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }
    fn evaluate_f64(&self, t: f64) -> (f64, f64) {
        let segs = self.segments();
        if segs.is_empty() {
            return (0.0, 0.0);
        }
        let (i, lt) = seg_param(segs.len(), t);
        segs[i].evaluate_f64(lt)
    }
    fn tangent_f64(&self, t: f64) -> (f64, f64) {
        let segs = self.segments();
        if segs.is_empty() {
            return (0.0, 0.0);
        }
        let (i, lt) = seg_param(segs.len(), t);
        segs[i].tangent_f64(lt)
    }
    fn bounding_box(&self) -> BoundingBox {
        let mut xmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymin = f64::INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        for p in &self.control {
            xmin = xmin.min(p.x);
            xmax = xmax.max(p.x);
            ymin = ymin.min(p.y);
            ymax = ymax.max(p.y);
        }
        BoundingBox::from_corners(xmin, ymin, xmax, ymax)
    }
    fn arc_length(&self) -> f64 {
        self.segments().iter().map(|s| s.arc_length()).sum()
    }
}
fn seg_param(n: usize, t: f64) -> (usize, f64) {
    let scaled = t.clamp(0.0, 1.0) * n as f64;
    let i = (scaled.floor() as usize).min(n - 1);
    (i, scaled - i as f64)
}

pub fn cv_spline_segments(cvs: &[Point2d]) -> Vec<RationalBezier> {
    cv_spline_segments_weighted(cvs, &vec![1.0; cvs.len()])
}

pub fn cv_spline_segments_weighted(cvs: &[Point2d], weights: &[f64]) -> Vec<RationalBezier> {
    match cvs.len() {
        0 | 1 => vec![],
        2..=4 => vec![RationalBezier::new(cvs.to_vec(), weights.to_vec())],
        _ => {
            let h: Vec<[f64; 3]> = cvs
                .iter()
                .zip(weights)
                .map(|(p, &w)| [w * p.x, w * p.y, w])
                .collect();
            clamped_cubic_bspline_homog(&h)
        }
    }
}

fn clamped_cubic_bspline_homog(h: &[[f64; 3]]) -> Vec<RationalBezier> {
    const P: usize = 3;
    let n = h.len() - 1;
    let interior = n - P;

    let mut knots: Vec<f64> = vec![0.0; P + 1];
    for i in 1..=interior {
        knots.push(i as f64);
    }
    knots.extend(std::iter::repeat_n((interior + 1) as f64, P + 1));

    let mut pts = h.to_vec();
    for k in 1..=interior {
        let val = k as f64;
        let mult = knots.iter().filter(|&&x| (x - val).abs() < 1e-9).count();
        for _ in mult..P {
            knot_insert_homog(&mut knots, &mut pts, val, P);
        }
    }

    (0..=interior)
        .map(|s| {
            let b = s * P;
            RationalBezier::from_homogeneous(&pts[b..b + 4])
        })
        .collect()
}

fn knot_insert_homog(knots: &mut Vec<f64>, pts: &mut Vec<[f64; 3]>, val: f64, p: usize) {
    let mut k = p;
    while k + 1 < knots.len() && !(knots[k] <= val && val < knots[k + 1]) {
        k += 1;
    }

    let mut out: Vec<[f64; 3]> = Vec::with_capacity(pts.len() + 1);
    out.extend_from_slice(&pts[..=k - p]);
    for i in (k - p + 1)..=k {
        let denom = knots[i + p] - knots[i];
        let a = if denom.abs() < 1e-12 {
            0.0
        } else {
            (val - knots[i]) / denom
        };
        let q = pts[i - 1];
        let r = pts[i];
        out.push([
            (1.0 - a) * q[0] + a * r[0],
            (1.0 - a) * q[1] + a * r[1],
            (1.0 - a) * q[2] + a * r[2],
        ]);
    }
    out.extend_from_slice(&pts[k..]);
    *pts = out;
    knots.insert(k + 1, val);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::CurveSegment;
    use crate::primitives::{CircularArc, CubicBezier, EllipticalArc, LineSeg};

    fn pt(x: f64, y: f64) -> Point2d {
        Point2d::from_f64(x, y)
    }

    #[test]
    fn line_lowers_to_degree_1() {
        let l = Curve::Line(LineSeg::from_endpoints(pt(1.0, 2.0), pt(5.0, 8.0)));
        let segs = lower(&l);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].degree(), 1);
        let m = segs[0].evaluate(0.25);
        assert!((m.x - 2.0).abs() < 1e-12 && (m.y - 3.5).abs() < 1e-12);
    }

    #[test]
    fn cubic_lowers_and_matches_evaluation() {
        let b = CubicBezier::new(pt(0.0, 0.0), pt(1.0, 3.0), pt(3.0, 3.0), pt(4.0, 0.0));
        let c = Curve::Bezier(b.clone());
        let segs = lower(&c);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].degree(), 3);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let (ex, ey) = b.evaluate_f64(t);
            let m = segs[0].evaluate(t);
            assert!(
                (m.x - ex).abs() < 1e-12 && (m.y - ey).abs() < 1e-12,
                "t={}",
                t
            );
        }
    }

    #[test]
    fn arc_lowers_to_exact_circle() {
        let a = CircularArc::new(pt(3.0, 4.0), 5.0, 0.0, 1.5 * std::f64::consts::PI);
        let segs = lower(&Curve::Arc(a));
        assert_eq!(
            segs.len(),
            3,
            "270° splits into three ≤90° rational quadratics"
        );
        for seg in &segs {
            assert_eq!(seg.degree(), 2);
            for i in 0..=16 {
                let p = seg.evaluate(i as f64 / 16.0);
                let d = ((p.x - 3.0).powi(2) + (p.y - 4.0).powi(2)).sqrt();
                assert!((d - 5.0).abs() < 1e-9, "off circle: d={}", d);
            }
        }
        let start = segs.first().unwrap().evaluate(0.0);
        let end = segs.last().unwrap().evaluate(1.0);
        assert!((start.x - 8.0).abs() < 1e-9 && (start.y - 4.0).abs() < 1e-9);
        assert!((end.x - 3.0).abs() < 1e-9 && (end.y - (4.0 - 5.0)).abs() < 1e-9);
    }

    #[test]
    fn quarter_arc_is_single_segment() {
        let a = CircularArc::new(pt(0.0, 0.0), 1.0, 0.0, std::f64::consts::FRAC_PI_2);
        let segs = lower(&Curve::Arc(a));
        assert_eq!(segs.len(), 1);
        let m = segs[0].evaluate(0.5);
        let inv = 1.0 / 2f64.sqrt();
        assert!(
            (m.x - inv).abs() < 1e-12 && (m.y - inv).abs() < 1e-12,
            "got {:?}",
            m
        );
    }

    #[test]
    fn ellipse_lowers_to_exact_ellipse() {
        let e = EllipticalArc::axis_aligned(pt(0.0, 0.0), 3.0, 2.0, 0.0, std::f64::consts::TAU);
        let segs = lower(&Curve::Ellipse(e));
        assert_eq!(segs.len(), 4);
        for seg in &segs {
            for i in 0..=16 {
                let p = seg.evaluate(i as f64 / 16.0);
                let f = (p.x / 3.0).powi(2) + (p.y / 2.0).powi(2);
                assert!((f - 1.0).abs() < 1e-9, "off ellipse: f={}", f);
            }
        }
    }

    #[test]
    fn rotated_ellipse_lowers_exactly() {
        let phi = 0.5;
        let (a, b) = (4.0_f64, 1.5_f64);
        let e = EllipticalArc::new(pt(1.0, -2.0), a, b, phi, 0.0, std::f64::consts::TAU);
        let segs = lower(&Curve::Ellipse(e));
        let (sin, cos) = phi.sin_cos();
        for seg in &segs {
            for i in 0..=12 {
                let p = seg.evaluate(i as f64 / 12.0);
                // Map back into the ellipse frame.
                let (dx, dy) = (p.x - 1.0, p.y + 2.0);
                let u = dx * cos + dy * sin;
                let v = -dx * sin + dy * cos;
                let f = (u / a).powi(2) + (v / b).powi(2);
                assert!((f - 1.0).abs() < 1e-9, "off rotated ellipse: f={}", f);
            }
        }
    }

    #[test]
    fn split_reconstructs_curve() {
        let a = CircularArc::new(pt(0.0, 0.0), 2.0, 0.0, std::f64::consts::FRAC_PI_2);
        let seg = lower(&Curve::Arc(a)).remove(0);
        let (left, right) = seg.split(0.5);
        let j0 = left.evaluate(1.0);
        let j1 = right.evaluate(0.0);
        assert!((j0.x - j1.x).abs() < 1e-12 && (j0.y - j1.y).abs() < 1e-12);
        for (c, s) in [(&left, 0.3), (&right, 0.7)] {
            let p = c.evaluate(s);
            let d = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (d - 2.0).abs() < 1e-9,
                "split half left the circle: d={}",
                d
            );
        }
    }

    #[test]
    fn tangent_matches_finite_difference() {
        let b = CubicBezier::new(pt(0.0, 0.0), pt(1.0, 3.0), pt(3.0, -1.0), pt(4.0, 0.0));
        let seg = lower(&Curve::Bezier(b)).remove(0);
        let h = 1e-6;
        for &t in &[0.2, 0.5, 0.8] {
            let (tx, ty) = seg.tangent(t);
            let a = seg.evaluate(t - h);
            let c = seg.evaluate(t + h);
            let (fx, fy) = ((c.x - a.x) / (2.0 * h), (c.y - a.y) / (2.0 * h));
            assert!((tx - fx).abs() < 1e-4 && (ty - fy).abs() < 1e-4, "t={}", t);
        }
    }

    #[test]
    fn cv_spline_four_points_is_single_cubic() {
        let cvs = vec![pt(0.0, 0.0), pt(1.0, 2.0), pt(3.0, 2.0), pt(4.0, 0.0)];
        let segs = cv_spline_segments(&cvs);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].degree(), 3);
        assert_eq!(segs[0].points, cvs, "4 CVs = the cubic Bézier through them");
    }

    #[test]
    fn cv_spline_clamped_cubic_bspline_properties() {
        let cvs = vec![
            pt(0.0, 0.0),
            pt(1.0, 3.0),
            pt(3.0, 3.0),
            pt(5.0, -1.0),
            pt(7.0, 2.0),
            pt(9.0, 0.0),
        ];
        let segs = cv_spline_segments(&cvs);
        assert_eq!(segs.len(), cvs.len() - 3, "6 CVs → 3 cubic spans");
        for s in &segs {
            assert_eq!(s.degree(), 3);
        }

        let start = segs.first().unwrap().evaluate(0.0);
        let end = segs.last().unwrap().evaluate(1.0);
        assert!(
            (start.x - 0.0).abs() < 1e-9 && (start.y - 0.0).abs() < 1e-9,
            "start {start:?}"
        );
        assert!(
            (end.x - 9.0).abs() < 1e-9 && (end.y - 0.0).abs() < 1e-9,
            "end {end:?}"
        );

        for w in segs.windows(2) {
            let a = w[0].evaluate(1.0);
            let b = w[1].evaluate(0.0);
            assert!(
                (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9,
                "join gap"
            );
            let (t0x, t0y) = w[0].tangent(1.0);
            let (t1x, t1y) = w[1].tangent(0.0);
            let cross = t0x * t1y - t0y * t1x;
            let dot = t0x * t1x + t0y * t1y;
            let mag = t0x.hypot(t0y) * t1x.hypot(t1y);
            assert!(
                cross.abs() < 1e-6 * mag.max(1.0) && dot > 0.0,
                "G1 break at joint"
            );
        }

        let (mut xmn, mut xmx, mut ymn, mut ymx) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for c in &cvs {
            xmn = xmn.min(c.x);
            xmx = xmx.max(c.x);
            ymn = ymn.min(c.y);
            ymx = ymx.max(c.y);
        }
        for s in &segs {
            for i in 0..=10 {
                let q = s.evaluate(i as f64 / 10.0);
                assert!(
                    q.x >= xmn - 1e-9
                        && q.x <= xmx + 1e-9
                        && q.y >= ymn - 1e-9
                        && q.y <= ymx + 1e-9,
                    "sample {q:?} outside control hull"
                );
            }
        }
    }

    #[test]
    fn nurbs_curve_clamped_with_uniform_weights() {
        let cvs = vec![
            pt(0.0, 0.0),
            pt(2.0, 4.0),
            pt(6.0, 4.0),
            pt(8.0, 0.0),
            pt(10.0, 4.0),
        ];
        let nc = NurbsCurve::uniform(cvs.clone());
        let s = nc.evaluate_f64(0.0);
        let e = nc.evaluate_f64(1.0);
        assert!(
            (s.0 - 0.0).abs() < 1e-9 && (s.1 - 0.0).abs() < 1e-9,
            "start {s:?}"
        );
        assert!(
            (e.0 - 10.0).abs() < 1e-9 && (e.1 - 4.0).abs() < 1e-9,
            "end {e:?}"
        );
        let bb = nc.bounding_box();
        assert!((bb.min.x - 0.0).abs() < 1e-9 && (bb.max.x - 10.0).abs() < 1e-9);
        assert_eq!(nc.segments(), cv_spline_segments(&cvs));
        assert!(nc.arc_length() > 0.0);
    }

    #[test]
    fn nurbs_weight_pulls_curve_toward_vertex() {
        let cvs = vec![
            pt(0.0, 0.0),
            pt(2.0, 4.0),
            pt(6.0, 4.0),
            pt(8.0, 0.0),
            pt(10.0, 4.0),
        ];
        let target = (6.0, 4.0); // cvs[2]
        let min_dist = |nc: &NurbsCurve| {
            (0..=40)
                .map(|i| {
                    let p = nc.evaluate_f64(i as f64 / 40.0);
                    ((p.0 - target.0).powi(2) + (p.1 - target.1).powi(2)).sqrt()
                })
                .fold(f64::MAX, f64::min)
        };

        let uniform = NurbsCurve::uniform(cvs.clone());
        let mut w = vec![1.0; cvs.len()];
        w[2] = 8.0;
        let heavy = NurbsCurve::new(cvs.clone(), w);
        assert!(
            min_dist(&heavy) < min_dist(&uniform),
            "raising weight[2] should pull the curve closer to cvs[2]"
        );
    }

    #[test]
    fn rational_is_a_first_class_curve() {
        let arc = CircularArc::new(pt(0.0, 0.0), 2.0, 0.0, std::f64::consts::FRAC_PI_2);
        let c = Curve::Rational(lower(&Curve::Arc(arc)).remove(0));

        let (x0, y0) = c.evaluate_f64(0.0);
        let (x1, y1) = c.evaluate_f64(1.0);
        assert!((x0 - 2.0).abs() < 1e-9 && y0.abs() < 1e-9);
        assert!(x1.abs() < 1e-9 && (y1 - 2.0).abs() < 1e-9);
        assert!(c.arc_length() > 0.0);

        assert_eq!(lower(&c).len(), 1);

        let (l, r) = crate::split_curve(&c, 0.5);
        for half in [&l, &r] {
            let (x, y) = half.evaluate_f64(0.5);
            assert!(
                (x.hypot(y) - 2.0).abs() < 1e-9,
                "split half left the circle"
            );
        }

        let (rx, ry) = crate::reverse_curve(&c).evaluate_f64(0.0);
        assert!(rx.abs() < 1e-9 && (ry - 2.0).abs() < 1e-9);

        let moved = crate::Transform2d::translation(10.0, 0.0).apply_curve(&c);
        assert!(matches!(moved, Curve::Rational(_)));
        let (mx, my) = moved.evaluate_f64(0.0);
        assert!((mx - 12.0).abs() < 1e-9 && my.abs() < 1e-9);
    }

    #[test]
    fn tessellate_circle_stays_on_circle() {
        let a = CircularArc::new(pt(0.0, 0.0), 10.0, 0.0, std::f64::consts::TAU);
        let poly = tessellate_curve(&Curve::Arc(a), 0.05);
        assert!(
            poly.len() > 8,
            "expected a refined polyline, got {}",
            poly.len()
        );
        for p in &poly {
            let d = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (d - 10.0).abs() < 1e-9,
                "tessellation vertex off circle: {}",
                d
            );
        }
    }
}
