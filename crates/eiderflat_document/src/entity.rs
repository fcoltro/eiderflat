use eiderflat_geometry::{Curve, CurveSegment, BoundingBox, Transform2d, Point2d};
use crate::properties::{Color, LineWeight, LineTypeRef, XData};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum HatchPattern {
    #[default]
    Solid,
    Lines { angle_deg: f64, spacing: f64 },
    Cross { angle_deg: f64, spacing: f64 },
    Dots { spacing: f64 },
}

#[derive(Clone, Debug)]
pub enum EntityKind {
    Curve(Curve),
    Point(Point2d),
    Text { anchor: Point2d, content: String, height: f64, rotation: f64, font: Option<String> },
    XLine { through: Point2d, dir: (f64, f64) },
    Ray { from: Point2d, dir: (f64, f64) },
    Insert { block: String, transform: Transform2d },
    Hatch { boundary: Vec<Curve>, holes: Vec<Vec<Curve>>, fill: (u8, u8, u8), pattern: HatchPattern },
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub layer: usize,
    pub color: Color,
    pub line_type: LineTypeRef,
    pub line_weight: LineWeight,
    pub transparency: f64,
    pub xdata: XData,
}

impl Entity {
    pub fn new(id: EntityId, kind: EntityKind, layer: usize) -> Self {
        Entity {
            id, kind, layer,
            color: Color::ByLayer,
            line_type: LineTypeRef::ByLayer,
            line_weight: LineWeight::ByLayer,
            transparency: 0.0,
            xdata: XData::default(),
        }
    }

    pub fn bounding_box(&self) -> Option<BoundingBox> {
        match &self.kind {
            EntityKind::Curve(c) => Some(c.bounding_box()),
            EntityKind::Point(p) => Some(BoundingBox::new(*p, *p)),
            EntityKind::Text { anchor, height, content, .. } => {
                let w = 0.6 * height * content.chars().count() as f64;
                let (ax, ay) = anchor.to_f64();
                Some(BoundingBox::from_corners(ax, ay, ax + w, ay + height))
            }
            EntityKind::Insert { .. } => None,
            EntityKind::XLine { .. } | EntityKind::Ray { .. } => None,
            EntityKind::Hatch { boundary, .. } => {
                boundary.iter().map(|c| c.bounding_box())
                    .reduce(|a, b| a.union(&b))
            }
        }
    }

    pub fn transform(&mut self, t: &Transform2d) {
        self.kind = match &self.kind {
            EntityKind::Curve(c) => EntityKind::Curve(t.apply_curve(c)),
            EntityKind::Point(p) => EntityKind::Point(t.apply_point(p)),
            EntityKind::Text { anchor, content, height, rotation, font } => EntityKind::Text {
                anchor: t.apply_point(anchor),
                content: content.clone(),
                height: height * t.scale_factor(),
                rotation: rotation + t.rotation_angle(),
                font: font.clone(),
            },
            EntityKind::XLine { through, dir } => EntityKind::XLine {
                through: t.apply_point(through),
                dir: transform_dir(t, dir),
            },
            EntityKind::Ray { from, dir } => EntityKind::Ray {
                from: t.apply_point(from),
                dir: transform_dir(t, dir),
            },
            EntityKind::Insert { block, transform } => EntityKind::Insert {
                block: block.clone(),
                transform: t.compose(transform),
            },
            EntityKind::Hatch { boundary, holes, fill, pattern } => EntityKind::Hatch {
                boundary: boundary.iter().map(|c| t.apply_curve(c)).collect(),
                holes: holes.iter().map(|h| h.iter().map(|c| t.apply_curve(c)).collect()).collect(),
                fill: *fill,
                pattern: transform_pattern(pattern, t),
            },
        };
    }

    pub fn transformed(&self, t: &Transform2d) -> Entity {
        let mut e = self.clone();
        e.transform(t);
        e
    }

    pub fn as_curve(&self) -> Option<&Curve> {
        if let EntityKind::Curve(c) = &self.kind { Some(c) } else { None }
    }
}

fn transform_pattern(p: &HatchPattern, t: &Transform2d) -> HatchPattern {
    let s = t.scale_factor();
    let rot = t.rotation_angle().to_degrees();
    match *p {
        HatchPattern::Solid => HatchPattern::Solid,
        HatchPattern::Lines { angle_deg, spacing } => {
            HatchPattern::Lines { angle_deg: angle_deg + rot, spacing: spacing * s }
        }
        HatchPattern::Cross { angle_deg, spacing } => {
            HatchPattern::Cross { angle_deg: angle_deg + rot, spacing: spacing * s }
        }
        HatchPattern::Dots { spacing } => HatchPattern::Dots { spacing: spacing * s },
    }
}

fn transform_dir(t: &Transform2d, dir: &(f64, f64)) -> (f64, f64) {
    let (dx, dy) = dir;
    (
        t.m00 * dx + t.m01 * dy,
        t.m10 * dx + t.m11 * dy,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eiderflat_geometry::LineSeg;

    fn pt(x: i64, y: i64) -> Point2d { Point2d::from_i64(x, y) }

    #[test]
    fn entity_bbox_for_line() {
        let line = Curve::Line(LineSeg::from_endpoints(pt(0, 0), pt(4, 3)));
        let e = Entity::new(EntityId(1), EntityKind::Curve(line), 0);
        let bb = e.bounding_box().unwrap();
        assert_eq!(bb.min, pt(0, 0));
        assert_eq!(bb.max, pt(4, 3));
    }

    #[test]
    fn move_entity_translates_geometry() {
        let line = Curve::Line(LineSeg::from_endpoints(pt(0, 0), pt(2, 0)));
        let mut e = Entity::new(EntityId(1), EntityKind::Curve(line), 0);
        e.transform(&Transform2d::translation(5.0, 3.0));
        let c = e.as_curve().unwrap();
        if let Curve::Line(l) = c {
            assert_eq!(l.p0, pt(5, 3));
            assert_eq!(l.p1, pt(7, 3));
        } else { panic!() }
    }

    #[test]
    fn transformed_keeps_original() {
        let line = Curve::Line(LineSeg::from_endpoints(pt(0, 0), pt(2, 0)));
        let e = Entity::new(EntityId(1), EntityKind::Curve(line), 0);
        let moved = e.transformed(&Transform2d::translation(10.0, 0.0));
        // Original unchanged
        if let Curve::Line(l) = e.as_curve().unwrap() { assert_eq!(l.p0, pt(0,0)); }
        // Copy moved
        if let Curve::Line(l) = moved.as_curve().unwrap() { assert_eq!(l.p0, pt(10,0)); }
    }

    #[test]
    fn infinite_lines_have_no_bbox() {
        let e = Entity::new(EntityId(1), EntityKind::XLine {
            through: pt(0,0), dir: (1.0, 0.0),
        }, 0);
        assert!(e.bounding_box().is_none());
    }
}
