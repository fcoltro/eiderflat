//! Construction solvers for tangent circles and circles through points: the
//! geometry behind the Tan-Tan-Radius, Tan-Tan-Tan, and 3-point circle tools.
//!
//! The tangent solvers work on the *locus of centers*: a circle of radius `r`
//! tangent to a line has its centre on one of the two lines parallel to it at
//! distance `r`; tangent to a circle of radius `R`, its centre lies on a
//! concentric circle of radius `R + r` (external) or `|R - r|` (internal).
//! Intersecting the loci of two targets yields the candidate centres.

use crate::curve::Curve;
use crate::point::Point2d;
use crate::primitives::{CircularArc, LineSeg};

const EPS: f64 = 1e-9;

/// Circle (centre, radius) through three points, or `None` if they're collinear.
pub fn circle_through_three_points(a: Point2d, b: Point2d, c: Point2d) -> Option<(Point2d, f64)> {
    CircularArc::from_three_points(&a, &b, &c).map(|arc| (arc.center, arc.radius))
}

/// Centre + radius of a circle of the given `radius` tangent to both curves,
/// choosing the solution whose centre is nearest `near`. Supports lines and
/// circular arcs (treated as their full lines / circles); returns `None` for
/// other curve kinds or when no tangent circle of that radius exists.
pub fn tangent_circle_ttr(
    c1: &Curve,
    c2: &Curve,
    radius: f64,
    near: Point2d,
) -> Option<(Point2d, f64)> {
    if radius <= EPS {
        return None;
    }
    let l1 = center_loci(c1, radius)?;
    let l2 = center_loci(c2, radius)?;
    let mut best: Option<(f64, Point2d)> = None;
    for a in &l1 {
        for b in &l2 {
            for p in intersect_loci(a, b) {
                let d = p.dist_sq(&near);
                if best.as_ref().map(|(bd, _)| d < *bd).unwrap_or(true) {
                    best = Some((d, p));
                }
            }
        }
    }
    best.map(|(_, c)| (c, radius))
}

/// Centre + radius of a circle tangent to three entities — the general
/// Apollonius "three objects" construction — choosing the solution whose centre
/// is nearest `near`. Each entity may be a line or a circle/arc (arcs are
/// treated as their full circle), in any mix. Returns `None` for other curve
/// kinds or when no positive-radius solution exists.
pub fn tangent_circle_ttt(
    c1: &Curve,
    c2: &Curve,
    c3: &Curve,
    near: Point2d,
) -> Option<(Point2d, f64)> {
    let objs = [as_object(c1)?, as_object(c2)?, as_object(c3)?];
    let mut best: Option<(f64, Point2d, f64)> = None;
    // Each object is tangent for the centre at signed offset ±r; try all eight
    // sign patterns (external/internal tangency to each) and keep r > 0.
    for &s0 in &[1.0, -1.0] {
        for &s1 in &[1.0, -1.0] {
            for &s2 in &[1.0, -1.0] {
                for (c, r) in solve_apollonius(&objs, [s0, s1, s2]) {
                    if r > EPS {
                        let d = c.dist_sq(&near);
                        if best.as_ref().map(|(bd, _, _)| d < *bd).unwrap_or(true) {
                            best = Some((d, c, r));
                        }
                    }
                }
            }
        }
    }
    best.map(|(_, c, r)| (c, r))
}

/// A tangency target reduced to its algebraic form.
#[derive(Clone, Copy)]
enum Object {
    /// Normalized line `a·x + b·y + c = 0` (unit normal ⇒ signed distance).
    Line {
        a: f64,
        b: f64,
        c: f64,
    },
    Circle {
        ox: f64,
        oy: f64,
        r: f64,
    },
}

fn as_object(curve: &Curve) -> Option<Object> {
    match curve {
        Curve::Line(l) => {
            let (a, b, c) = line_equation(l);
            Some(Object::Line { a, b, c })
        }
        Curve::Arc(arc) => {
            let (ox, oy) = arc.center.to_f64();
            Some(Object::Circle {
                ox,
                oy,
                r: arc.radius,
            })
        }
        _ => None,
    }
}

/// Solve for circles tangent to the three objects under one sign pattern
/// (`signs[i]` = +1 external / −1 internal tangency to object `i`).
///
/// A line gives the linear equation `a·x + b·y − s·r = −c`. A circle gives the
/// quadratic `(x−ox)² + (y−oy)² = (R + s·r)²`; subtracting one circle's
/// equation from another cancels the `x²+y²` terms, leaving a linear equation.
/// So with at least one circle we keep one circle as the quadratic "anchor",
/// reduce the other two constraints to two linear equations in `(x, y, r)`,
/// express `x` and `y` affinely in `r`, and substitute into the anchor to get a
/// quadratic in `r`. With no circle it is a plain 3×3 linear system.
fn solve_apollonius(objs: &[Object; 3], signs: [f64; 3]) -> Vec<(Point2d, f64)> {
    let anchor = objs.iter().position(|o| matches!(o, Object::Circle { .. }));

    let Some(ai) = anchor else {
        // All three are lines: one linear system, one solution.
        let mut rows = [[0.0; 4]; 3];
        for i in 0..3 {
            if let Object::Line { a, b, c } = objs[i] {
                rows[i] = [a, b, -signs[i], -c];
            }
        }
        return match solve_3x3(rows) {
            Some([x, y, r]) => vec![(Point2d::from_f64(x, y), r)],
            None => Vec::new(),
        };
    };

    let Object::Circle {
        ox: oax,
        oy: oay,
        r: ra,
    } = objs[ai]
    else {
        unreachable!()
    };
    let sa = signs[ai];
    let ka = oax * oax + oay * oay - ra * ra;

    // Two linear equations [α, β, γ, δ] meaning α·x + β·y + γ·r = δ.
    let mut eqs: Vec<[f64; 4]> = Vec::with_capacity(2);
    for k in 0..3 {
        if k == ai {
            continue;
        }
        match objs[k] {
            Object::Line { a, b, c } => eqs.push([a, b, -signs[k], -c]),
            Object::Circle { ox, oy, r: rk } => {
                let kk = ox * ox + oy * oy - rk * rk;
                eqs.push([
                    2.0 * (oax - ox),
                    2.0 * (oay - oy),
                    2.0 * (ra * sa - rk * signs[k]),
                    ka - kk,
                ]);
            }
        }
    }

    let [a1, b1, g1, d1] = eqs[0];
    let [a2, b2, g2, d2] = eqs[1];
    let det = a1 * b2 - a2 * b1;
    if det.abs() < EPS {
        return Vec::new();
    }
    // x = x0 + xr·r,  y = y0 + yr·r.
    let x0 = (b2 * d1 - b1 * d2) / det;
    let xr = -(b2 * g1 - b1 * g2) / det;
    let y0 = (a1 * d2 - a2 * d1) / det;
    let yr = -(a1 * g2 - a2 * g1) / det;

    // Substitute into (x−oax)² + (y−oay)² = (ra + sa·r)².
    let (p0, p1) = (x0 - oax, xr);
    let (q0, q1) = (y0 - oay, yr);
    let qa = p1 * p1 + q1 * q1 - 1.0;
    let qb = 2.0 * (p0 * p1 + q0 * q1) - 2.0 * ra * sa;
    let qc = p0 * p0 + q0 * q0 - ra * ra;

    solve_quadratic(qa, qb, qc)
        .into_iter()
        .map(|r| (Point2d::from_f64(x0 + xr * r, y0 + yr * r), r))
        .collect()
}

/// Real roots of `a·x² + b·x + c = 0`, including the linear case `a ≈ 0`.
fn solve_quadratic(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a.abs() < EPS {
        if b.abs() < EPS {
            return Vec::new();
        }
        return vec![-c / b];
    }
    let disc = b * b - 4.0 * a * c;
    if disc < -EPS {
        return Vec::new();
    }
    let s = disc.max(0.0).sqrt();
    vec![(-b + s) / (2.0 * a), (-b - s) / (2.0 * a)]
}

/// The points on the circle (centre `o`, radius `r`) where a line from external
/// point `p` is tangent. Two points when `p` is outside, one when on the circle,
/// none when inside.
pub fn tangent_points_from_point(o: Point2d, r: f64, p: Point2d) -> Vec<Point2d> {
    let d = o.dist_f64(&p);
    if d < r - EPS || r <= EPS {
        return Vec::new();
    }
    let base = (p.y - o.y).atan2(p.x - o.x);
    let th = (r / d).clamp(-1.0, 1.0).acos();
    if th < EPS {
        return vec![Point2d::from_f64(
            o.x + r * base.cos(),
            o.y + r * base.sin(),
        )];
    }
    [base + th, base - th]
        .iter()
        .map(|a| Point2d::from_f64(o.x + r * a.cos(), o.y + r * a.sin()))
        .collect()
}

/// Common tangent segments between two circles, each as the pair of touch points
/// `(on circle 1, on circle 2)`. Returns the two outer tangents (when they
/// exist) followed by the two inner tangents (when the circles are separate).
pub fn common_tangent_segments(
    o1: Point2d,
    r1: f64,
    o2: Point2d,
    r2: f64,
) -> Vec<(Point2d, Point2d)> {
    let (dx, dy) = (o2.x - o1.x, o2.y - o1.y);
    let d = (dx * dx + dy * dy).sqrt();
    if d < EPS {
        return Vec::new();
    }
    let (ux, uy) = (dx / d, dy / d); // along the centre line
    let (vx, vy) = (-uy, ux); // perpendicular
    let mut out = Vec::new();
    // s1 fixed at +1; s2 = +1 gives the outer pair, s2 = -1 the inner pair. The
    // ± on the perpendicular component gives the two lines of each pair.
    for &s2 in &[1.0_f64, -1.0] {
        let k = s2 * r2 - r1;
        let along = k / d;
        if along.abs() > 1.0 + EPS {
            continue;
        }
        let perp = (1.0 - along * along).max(0.0).sqrt();
        for &sign in &[1.0_f64, -1.0] {
            // Line normal n = along·û + sign·perp·v̂ (unit).
            let nx = along * ux + sign * perp * vx;
            let ny = along * uy + sign * perp * vy;
            let t1 = Point2d::from_f64(o1.x - r1 * nx, o1.y - r1 * ny);
            let t2 = Point2d::from_f64(o2.x - s2 * r2 * nx, o2.y - s2 * r2 * ny);
            out.push((t1, t2));
        }
    }
    out
}

// ── Locus of centres ────────────────────────────────────────────────────────

/// A locus on which a tangent circle's centre can lie.
enum Locus {
    /// Infinite line through `p` with unit direction `d`.
    Line { p: (f64, f64), d: (f64, f64) },
    /// Full circle.
    Circle { c: (f64, f64), r: f64 },
}

/// The (one or two) loci of centres for circles of radius `dist` tangent to the
/// curve. `None` for unsupported curve kinds.
fn center_loci(curve: &Curve, dist: f64) -> Option<Vec<Locus>> {
    match curve {
        Curve::Line(l) => {
            let (dx, dy) = l.direction();
            let len = (dx * dx + dy * dy).sqrt();
            if len < EPS {
                return None;
            }
            let (ux, uy) = (dx / len, dy / len);
            let (nx, ny) = (-uy, ux); // unit normal
            let (x0, y0) = l.p0.to_f64();
            Some(vec![
                Locus::Line {
                    p: (x0 + nx * dist, y0 + ny * dist),
                    d: (ux, uy),
                },
                Locus::Line {
                    p: (x0 - nx * dist, y0 - ny * dist),
                    d: (ux, uy),
                },
            ])
        }
        Curve::Arc(a) => {
            let c = a.center.to_f64();
            let mut loci = vec![Locus::Circle {
                c,
                r: a.radius + dist,
            }];
            let inner = (a.radius - dist).abs();
            if inner > EPS {
                loci.push(Locus::Circle { c, r: inner });
            }
            Some(loci)
        }
        _ => None,
    }
}

fn intersect_loci(a: &Locus, b: &Locus) -> Vec<Point2d> {
    match (a, b) {
        (Locus::Line { p: p1, d: d1 }, Locus::Line { p: p2, d: d2 }) => {
            line_line(*p1, *d1, *p2, *d2).into_iter().collect()
        }
        (Locus::Line { p, d }, Locus::Circle { c, r })
        | (Locus::Circle { c, r }, Locus::Line { p, d }) => line_circle(*p, *d, *c, *r),
        (Locus::Circle { c: c1, r: r1 }, Locus::Circle { c: c2, r: r2 }) => {
            circle_circle(*c1, *r1, *c2, *r2)
        }
    }
}

// ── Analytic intersections of infinite lines / full circles ─────────────────

fn line_line(p1: (f64, f64), d1: (f64, f64), p2: (f64, f64), d2: (f64, f64)) -> Option<Point2d> {
    let denom = d1.0 * d2.1 - d1.1 * d2.0;
    if denom.abs() < EPS {
        return None; // parallel
    }
    let (rx, ry) = (p2.0 - p1.0, p2.1 - p1.1);
    let t = (rx * d2.1 - ry * d2.0) / denom;
    Some(Point2d::from_f64(p1.0 + t * d1.0, p1.1 + t * d1.1))
}

fn line_circle(p: (f64, f64), d: (f64, f64), c: (f64, f64), r: f64) -> Vec<Point2d> {
    // Closest point on the line to the circle centre, then step ±along the line.
    let dd = d.0 * d.0 + d.1 * d.1;
    if dd < EPS {
        return Vec::new();
    }
    let t = ((c.0 - p.0) * d.0 + (c.1 - p.1) * d.1) / dd;
    let foot = (p.0 + t * d.0, p.1 + t * d.1);
    let dist2 = (foot.0 - c.0).powi(2) + (foot.1 - c.1).powi(2);
    let h2 = r * r - dist2;
    if h2 < -EPS {
        return Vec::new();
    }
    let h = h2.max(0.0).sqrt() / dd.sqrt();
    let (ux, uy) = (d.0 / dd.sqrt(), d.1 / dd.sqrt());
    if h < EPS {
        vec![Point2d::from_f64(foot.0, foot.1)]
    } else {
        vec![
            Point2d::from_f64(foot.0 + ux * h, foot.1 + uy * h),
            Point2d::from_f64(foot.0 - ux * h, foot.1 - uy * h),
        ]
    }
}

fn circle_circle(c1: (f64, f64), r1: f64, c2: (f64, f64), r2: f64) -> Vec<Point2d> {
    let (dx, dy) = (c2.0 - c1.0, c2.1 - c1.1);
    let d = (dx * dx + dy * dy).sqrt();
    if d < EPS || d > r1 + r2 + EPS || d < (r1 - r2).abs() - EPS {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h2 = r1 * r1 - a * a;
    let h = h2.max(0.0).sqrt();
    let (mx, my) = (c1.0 + a * dx / d, c1.1 + a * dy / d);
    if h < EPS {
        vec![Point2d::from_f64(mx, my)]
    } else {
        let (ox, oy) = (-dy / d * h, dx / d * h);
        vec![
            Point2d::from_f64(mx + ox, my + oy),
            Point2d::from_f64(mx - ox, my - oy),
        ]
    }
}

// ── Small helpers ───────────────────────────────────────────────────────────

/// `(a, b, c)` for the normalized implicit line equation `a·x + b·y + c = 0`,
/// with `(a, b)` a unit normal so the expression is signed distance.
fn line_equation(l: &LineSeg) -> (f64, f64, f64) {
    let (dx, dy) = l.direction();
    let len = (dx * dx + dy * dy).sqrt().max(EPS);
    let (a, b) = (-dy / len, dx / len);
    let (x0, y0) = l.p0.to_f64();
    (a, b, -(a * x0 + b * y0))
}

/// Solve a 3×3 system given as rows `[a, b, c, rhs]` by Cramer's rule.
fn solve_3x3(m: [[f64; 4]; 3]) -> Option<[f64; 3]> {
    let det3 = |a: [[f64; 3]; 3]| {
        a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
    };
    let base = [
        [m[0][0], m[0][1], m[0][2]],
        [m[1][0], m[1][1], m[1][2]],
        [m[2][0], m[2][1], m[2][2]],
    ];
    let det = det3(base);
    if det.abs() < EPS {
        return None;
    }
    let mut out = [0.0; 3];
    for (k, slot) in out.iter_mut().enumerate() {
        let mut a = base;
        for r in 0..3 {
            a[r][k] = m[r][3];
        }
        *slot = det3(a) / det;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> Curve {
        Curve::Line(LineSeg::from_endpoints(
            Point2d::from_f64(x0, y0),
            Point2d::from_f64(x1, y1),
        ))
    }

    fn circle(cx: f64, cy: f64, r: f64) -> Curve {
        Curve::Arc(CircularArc::new(
            Point2d::from_f64(cx, cy),
            r,
            0.0,
            std::f64::consts::TAU,
        ))
    }

    /// True if the circle (centre `c`, radius `r`) is tangent to the object.
    fn is_tangent(c: Point2d, r: f64, obj: &Curve) -> bool {
        match obj {
            Curve::Line(l) => {
                let (a, b, cc) = line_equation(l);
                ((a * c.x + b * c.y + cc).abs() - r).abs() < 1e-6
            }
            Curve::Arc(arc) => {
                let (ox, oy) = arc.center.to_f64();
                let d = ((c.x - ox).powi(2) + (c.y - oy).powi(2)).sqrt();
                (d - (arc.radius + r)).abs() < 1e-6 || (d - (arc.radius - r).abs()).abs() < 1e-6
            }
            _ => false,
        }
    }

    #[test]
    fn ttr_two_axes_picks_quadrant_by_near() {
        // x-axis and y-axis; radius 1 → centre at (±1, ±1). The `near` point
        // selects the quadrant.
        let x = line(0.0, 0.0, 10.0, 0.0);
        let y = line(0.0, 0.0, 0.0, 10.0);
        let (c, r) = tangent_circle_ttr(&x, &y, 1.0, Point2d::from_f64(5.0, 5.0)).unwrap();
        assert!((r - 1.0).abs() < 1e-9);
        assert!(
            (c.x - 1.0).abs() < 1e-9 && (c.y - 1.0).abs() < 1e-9,
            "got {c:?}"
        );
        let (c2, _) = tangent_circle_ttr(&x, &y, 1.0, Point2d::from_f64(-5.0, -5.0)).unwrap();
        assert!(
            (c2.x + 1.0).abs() < 1e-9 && (c2.y + 1.0).abs() < 1e-9,
            "got {c2:?}"
        );
    }

    #[test]
    fn ttr_tangent_to_circle_and_line() {
        // Circle centre (0,0) r=2, and the line y=0. A radius-1 circle tangent
        // to both, above the line and outside the circle, sits at distance 3
        // from origin and 1 above the x-axis → centre y=1, x=±√8.
        let circ = Curve::Arc(CircularArc::new(
            Point2d::from_f64(0.0, 0.0),
            2.0,
            0.0,
            std::f64::consts::TAU,
        ));
        let l = line(-10.0, 0.0, 10.0, 0.0);
        let (c, r) = tangent_circle_ttr(&circ, &l, 1.0, Point2d::from_f64(3.0, 1.0)).unwrap();
        assert!((r - 1.0).abs() < 1e-9);
        assert!((c.y - 1.0).abs() < 1e-6, "centre y should be 1, got {c:?}");
        assert!(
            ((c.x * c.x + c.y * c.y).sqrt() - 3.0).abs() < 1e-6,
            "dist to origin 3, got {c:?}"
        );
    }

    #[test]
    fn ttt_incircle_of_right_triangle() {
        // Legs on the axes from (0,0)-(6,0) and (0,0)-(0,6), hypotenuse
        // (6,0)-(0,6). Incircle radius r = (a + b − c)/2 = (6+6−6√2)/2.
        let a = line(0.0, 0.0, 6.0, 0.0);
        let b = line(0.0, 0.0, 0.0, 6.0);
        let c = line(6.0, 0.0, 0.0, 6.0);
        let (center, r) = tangent_circle_ttt(&a, &b, &c, Point2d::from_f64(1.5, 1.5)).unwrap();
        let expect = (12.0 - 6.0 * 2.0_f64.sqrt()) / 2.0;
        assert!((r - expect).abs() < 1e-6, "incircle r {expect}, got {r}");
        assert!(
            (center.x - r).abs() < 1e-6 && (center.y - r).abs() < 1e-6,
            "got {center:?}"
        );
    }

    #[test]
    fn ttt_three_circles_inner_soddy() {
        // Three unit circles at the vertices of an equilateral triangle (side 4).
        // The solution near the centroid is tangent to all three.
        let s = 3.0_f64.sqrt();
        let a = circle(0.0, 0.0, 1.0);
        let b = circle(4.0, 0.0, 1.0);
        let c = circle(2.0, 2.0 * s, 1.0);
        let near = Point2d::from_f64(2.0, 2.0 * s / 3.0);
        let (center, r) = tangent_circle_ttt(&a, &b, &c, near).unwrap();
        assert!(r > 0.0);
        assert!(
            is_tangent(center, r, &a),
            "not tangent to a: {center:?} r={r}"
        );
        assert!(is_tangent(center, r, &b), "not tangent to b");
        assert!(is_tangent(center, r, &c), "not tangent to c");
    }

    #[test]
    fn ttt_two_lines_and_a_circle() {
        // The two axes and a circle out on the diagonal. The solution hugging the
        // corner is tangent to both axes and the circle.
        let x = line(0.0, 0.0, 10.0, 0.0);
        let y = line(0.0, 0.0, 0.0, 10.0);
        let circ = circle(6.0, 6.0, 2.0);
        let (center, r) = tangent_circle_ttt(&x, &y, &circ, Point2d::from_f64(1.5, 1.5)).unwrap();
        assert!(r > 0.0);
        assert!(is_tangent(center, r, &x), "not tangent to x-axis");
        assert!(is_tangent(center, r, &y), "not tangent to y-axis");
        assert!(is_tangent(center, r, &circ), "not tangent to circle");
    }

    #[test]
    fn ttt_line_and_two_circles() {
        let l = line(-10.0, 0.0, 10.0, 0.0);
        let c1 = circle(-3.0, 4.0, 1.5);
        let c2 = circle(3.0, 4.0, 1.5);
        let (center, r) = tangent_circle_ttt(&l, &c1, &c2, Point2d::from_f64(0.0, 3.0)).unwrap();
        assert!(r > 0.0);
        assert!(is_tangent(center, r, &l), "not tangent to line");
        assert!(is_tangent(center, r, &c1), "not tangent to c1");
        assert!(is_tangent(center, r, &c2), "not tangent to c2");
    }

    #[test]
    fn tangent_points_from_external_point() {
        // Unit circle at origin, point at (2,0): tangent touch points are at
        // x = 1/2, y = ±√3/2 (the classic 60° result).
        let pts = tangent_points_from_point(
            Point2d::from_f64(0.0, 0.0),
            1.0,
            Point2d::from_f64(2.0, 0.0),
        );
        assert_eq!(pts.len(), 2);
        for p in &pts {
            assert!((p.x - 0.5).abs() < 1e-9, "touch x should be 0.5, got {p:?}");
            assert!((p.y.abs() - 3.0_f64.sqrt() / 2.0).abs() < 1e-9);
            // Touch point lies on the circle.
            assert!(((p.x * p.x + p.y * p.y).sqrt() - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn point_inside_circle_has_no_tangents() {
        let pts = tangent_points_from_point(
            Point2d::from_f64(0.0, 0.0),
            2.0,
            Point2d::from_f64(0.5, 0.0),
        );
        assert!(pts.is_empty());
    }

    #[test]
    fn common_tangents_of_two_equal_circles() {
        // Two unit circles at (0,0) and (4,0): 2 outer + 2 inner = 4 tangents.
        let segs = common_tangent_segments(
            Point2d::from_f64(0.0, 0.0),
            1.0,
            Point2d::from_f64(4.0, 0.0),
            1.0,
        );
        assert_eq!(segs.len(), 4);
        // Outer tangents are horizontal at y = ±1; each touch point on its circle.
        for (a, b) in &segs {
            assert!(((a.x).powi(2) + (a.y).powi(2)).sqrt() - 1.0 < 1e-9);
            assert!(((b.x - 4.0).powi(2) + (b.y).powi(2)).sqrt() - 1.0 < 1e-9);
            // The segment is perpendicular to both radii (tangent): direction ·
            // radius vector ≈ 0 at each end.
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let len = (dx * dx + dy * dy).sqrt().max(1e-12);
            let dot_a = (dx / len) * (a.x - 0.0) + (dy / len) * (a.y - 0.0);
            assert!(dot_a.abs() < 1e-6, "segment not tangent at circle 1");
        }
    }

    #[test]
    fn nested_circles_have_no_common_tangents() {
        // A small circle entirely inside a big one shares no tangent line.
        let segs = common_tangent_segments(
            Point2d::from_f64(0.0, 0.0),
            5.0,
            Point2d::from_f64(0.5, 0.0),
            1.0,
        );
        assert!(segs.is_empty());
    }

    #[test]
    fn three_point_circle_is_unit_circle() {
        let (c, r) = circle_through_three_points(
            Point2d::from_f64(1.0, 0.0),
            Point2d::from_f64(0.0, 1.0),
            Point2d::from_f64(-1.0, 0.0),
        )
        .unwrap();
        assert!(c.x.abs() < 1e-9 && c.y.abs() < 1e-9 && (r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn collinear_three_points_have_no_circle() {
        assert!(
            circle_through_three_points(
                Point2d::from_f64(0.0, 0.0),
                Point2d::from_f64(1.0, 1.0),
                Point2d::from_f64(2.0, 2.0),
            )
            .is_none()
        );
    }
}
