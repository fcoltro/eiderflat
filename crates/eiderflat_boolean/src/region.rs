use eiderflat_geometry::{Curve, CurveSegment, Point2d, tessellate_curve};
use robust::{Coord, orient2d};

#[derive(Clone, Debug)]
pub struct Region {
    pub outer: Vec<Curve>,
    pub holes: Vec<Vec<Curve>>,
}

impl Region {
    pub fn new(outer: Vec<Curve>) -> Self {
        Region {
            outer,
            holes: Vec::new(),
        }
    }

    pub fn with_holes(outer: Vec<Curve>, holes: Vec<Vec<Curve>>) -> Self {
        Region { outer, holes }
    }

    pub fn signed_area_f64(&self) -> f64 {
        boundary_signed_area(&self.outer)
            - self
                .holes
                .iter()
                .map(|h| boundary_signed_area(h).abs())
                .sum::<f64>()
    }

    pub fn winding_number(&self, px: f64, py: f64) -> i32 {
        let mut wn = winding_number_boundary(&self.outer, px, py);
        for hole in &self.holes {
            wn += winding_number_boundary(hole, px, py);
        }
        wn
    }

    pub fn contains_point(&self, px: f64, py: f64) -> bool {
        self.winding_number(px, py) != 0
    }
}

fn flatten_segment(seg: &Curve) -> Vec<Point2d> {
    let bb = seg.bounding_box();
    let diag = ((bb.max.x - bb.min.x).powi(2) + (bb.max.y - bb.min.y).powi(2)).sqrt();
    let tol = (diag * 1e-4).max(1e-9);
    tessellate_curve(seg, tol)
}

/// Flattens every boundary segment into one continuous vertex ring. Shared
/// endpoints between consecutive segments are de-duplicated so the ring carries
/// no zero-length edges, and the ring is meant to be read as *closed*: the edge
/// from the last vertex back to the first must be walked too.
///
/// Closing the ring matters because a full-circle arc tessellates to a polyline
/// whose ends differ by a rounding gap (`sin(2π) ≈ -1.2e-16`, not 0). Walking
/// only the within-segment edges drops the crossing that lives in that seam, so
/// a horizontal ray whose `y` lands inside the gap miscounts and reports an
/// outside point as inside.
fn boundary_ring(boundary: &[Curve]) -> Vec<Point2d> {
    let mut ring: Vec<Point2d> = Vec::new();
    for seg in boundary {
        let poly = flatten_segment(seg);
        let mut iter = poly.into_iter();
        if let Some(first) = iter.next() {
            // Skip the first point when it coincides with the previous segment's end.
            if ring
                .last()
                .is_none_or(|l| (l.x - first.x).abs() > 1e-12 || (l.y - first.y).abs() > 1e-12)
            {
                ring.push(first);
            }
            ring.extend(iter);
        }
    }
    // Drop a trailing point that coincides with the start: the closing edge added
    // by the wraparound walk would otherwise be zero-length (and the seam crossing
    // would still be missed).
    if ring.len() >= 2 {
        let (f, l) = (ring[0], *ring.last().unwrap());
        if (f.x - l.x).abs() <= 1e-12 && (f.y - l.y).abs() <= 1e-12 {
            ring.pop();
        }
    }
    ring
}

fn boundary_signed_area(boundary: &[Curve]) -> f64 {
    let ring = boundary_ring(boundary);
    let n = ring.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        area += (a.x + b.x) * (b.y - a.y);
    }
    area / 2.0
}

fn winding_number_boundary(boundary: &[Curve], px: f64, py: f64) -> i32 {
    let ring = boundary_ring(boundary);
    let n = ring.len();
    if n < 3 {
        return 0;
    }
    let mut wn = 0i32;
    for i in 0..n {
        let (x1, y1) = (ring[i].x, ring[i].y);
        let (x2, y2) = (ring[(i + 1) % n].x, ring[(i + 1) % n].y);
        if y1 <= py {
            if y2 > py && cross_sign(x1, y1, x2, y2, px, py) > 0.0 {
                wn += 1;
            }
        } else if y2 <= py && cross_sign(x1, y1, x2, y2, px, py) < 0.0 {
            wn -= 1;
        }
    }
    wn
}

fn cross_sign(x1: f64, y1: f64, x2: f64, y2: f64, px: f64, py: f64) -> f64 {
    orient2d(
        Coord { x: x1, y: y1 },
        Coord { x: x2, y: y2 },
        Coord { x: px, y: py },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eiderflat_geometry::{CircularArc, Curve, LineSeg, Point2d};

    fn square_region() -> Region {
        Region::new(vec![
            Curve::Line(LineSeg::from_endpoints(
                Point2d::from_i64(0, 0),
                Point2d::from_i64(2, 0),
            )),
            Curve::Line(LineSeg::from_endpoints(
                Point2d::from_i64(2, 0),
                Point2d::from_i64(2, 2),
            )),
            Curve::Line(LineSeg::from_endpoints(
                Point2d::from_i64(2, 2),
                Point2d::from_i64(0, 2),
            )),
            Curve::Line(LineSeg::from_endpoints(
                Point2d::from_i64(0, 2),
                Point2d::from_i64(0, 0),
            )),
        ])
    }

    #[test]
    fn interior_point() {
        let r = square_region();
        assert!(r.contains_point(1.0, 1.0));
    }

    #[test]
    fn exterior_point() {
        let r = square_region();
        assert!(!r.contains_point(5.0, 5.0));
    }

    #[test]
    fn signed_area_positive_ccw() {
        let r = square_region();
        let area = r.signed_area_f64();
        assert!(
            area > 0.0,
            "CCW boundary should have positive area, got {}",
            area
        );
        assert!((area - 4.0).abs() < 0.1, "area≈{}", area);
    }

    #[test]
    fn circle_region_area_and_classification() {
        let r = Region::new(vec![Curve::Arc(CircularArc::new(
            Point2d::from_i64(0, 0),
            3.0,
            0.0,
            std::f64::consts::TAU,
        ))]);
        let area = r.signed_area_f64();
        let expected = std::f64::consts::PI * 9.0;
        assert!(
            (area - expected).abs() < 1e-2,
            "circle area ≈ {expected}, got {area}"
        );
        assert!(r.contains_point(0.0, 0.0), "centre is inside");
        assert!(r.contains_point(2.9, 0.0), "just inside the rim");
        assert!(!r.contains_point(3.1, 0.0), "just outside the rim");
        assert!(!r.contains_point(10.0, 10.0), "far point is outside");
    }

    #[test]
    fn full_circle_seam_does_not_leak_winding() {
        // A full-circle arc tessellates to a polyline whose ends differ by a
        // rounding gap (sin(2π) ≈ -1.2e-16). A horizontal ray whose y lands inside
        // that seam must still see both rim crossings; before closing the ring, the
        // seam crossing was dropped and a far-outside point read as inside, leaking
        // hatch lines beyond the circle.
        let r = Region::new(vec![Curve::Arc(CircularArc::new(
            Point2d::from_i64(0, 0),
            5.0,
            0.0,
            std::f64::consts::TAU,
        ))]);
        for &y in &[0.0, -0.0, 1e-16, -1e-16, -1e-300, 1e-9, -1e-9] {
            assert!(
                !r.contains_point(-9.571, y),
                "point far left of the circle must be outside (y={y:+e})"
            );
            assert!(
                !r.contains_point(9.571, y),
                "point far right of the circle must be outside (y={y:+e})"
            );
            assert!(
                r.contains_point(0.0, y),
                "the centre line must be inside (y={y:+e})"
            );
        }
    }

    #[test]
    fn rotated_diamond_classification_uses_robust_orientation() {
        let d = Region::new(vec![
            Curve::Line(LineSeg::from_endpoints(
                Point2d::from_i64(3, 0),
                Point2d::from_i64(0, 3),
            )),
            Curve::Line(LineSeg::from_endpoints(
                Point2d::from_i64(0, 3),
                Point2d::from_i64(-3, 0),
            )),
            Curve::Line(LineSeg::from_endpoints(
                Point2d::from_i64(-3, 0),
                Point2d::from_i64(0, -3),
            )),
            Curve::Line(LineSeg::from_endpoints(
                Point2d::from_i64(0, -3),
                Point2d::from_i64(3, 0),
            )),
        ]);
        assert!(d.contains_point(0.0, 0.0), "centre inside");
        assert!(d.contains_point(1.4, 1.4), "just inside the x+y=3 edge");
        assert!(!d.contains_point(1.6, 1.6), "just outside the x+y=3 edge");
        assert!(!d.contains_point(2.0, 2.0), "corner-diagonal point outside");
    }
}
