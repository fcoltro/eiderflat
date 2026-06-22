use crate::nurbs::{NurbsCurve, RationalBezier};
use crate::point::BoundingBox;
use crate::primitives::{CircularArc, CubicBezier, EllipticalArc, LineSeg, PolyCurve};

pub trait CurveSegment {
    fn domain(&self) -> (f64, f64);

    fn evaluate_f64(&self, t: f64) -> (f64, f64);

    fn bounding_box(&self) -> BoundingBox;

    fn tangent_f64(&self, t: f64) -> (f64, f64);

    fn normal_f64(&self, t: f64) -> (f64, f64) {
        let (tx, ty) = self.tangent_f64(t);
        (-ty, tx)
    }

    fn arc_length(&self) -> f64;
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Curve {
    Line(LineSeg),
    Arc(CircularArc),
    Ellipse(EllipticalArc),
    Bezier(CubicBezier),
    Poly(Box<PolyCurve>),
    Rational(RationalBezier),
    Nurbs(NurbsCurve),
}

impl Curve {
    pub fn as_line(&self) -> Option<&LineSeg> {
        if let Curve::Line(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl CurveSegment for Curve {
    fn domain(&self) -> (f64, f64) {
        match self {
            Curve::Line(v) => v.domain(),
            Curve::Arc(v) => v.domain(),
            Curve::Ellipse(v) => v.domain(),
            Curve::Bezier(v) => v.domain(),
            Curve::Poly(v) => v.domain(),
            Curve::Rational(v) => v.domain(),
            Curve::Nurbs(v) => v.domain(),
        }
    }
    fn evaluate_f64(&self, t: f64) -> (f64, f64) {
        match self {
            Curve::Line(v) => v.evaluate_f64(t),
            Curve::Arc(v) => v.evaluate_f64(t),
            Curve::Ellipse(v) => v.evaluate_f64(t),
            Curve::Bezier(v) => v.evaluate_f64(t),
            Curve::Poly(v) => v.evaluate_f64(t),
            Curve::Rational(v) => v.evaluate_f64(t),
            Curve::Nurbs(v) => v.evaluate_f64(t),
        }
    }
    fn bounding_box(&self) -> BoundingBox {
        match self {
            Curve::Line(v) => v.bounding_box(),
            Curve::Arc(v) => v.bounding_box(),
            Curve::Ellipse(v) => v.bounding_box(),
            Curve::Bezier(v) => v.bounding_box(),
            Curve::Poly(v) => v.bounding_box(),
            Curve::Rational(v) => v.bounding_box(),
            Curve::Nurbs(v) => v.bounding_box(),
        }
    }
    fn tangent_f64(&self, t: f64) -> (f64, f64) {
        match self {
            Curve::Line(v) => v.tangent_f64(t),
            Curve::Arc(v) => v.tangent_f64(t),
            Curve::Ellipse(v) => v.tangent_f64(t),
            Curve::Bezier(v) => v.tangent_f64(t),
            Curve::Poly(v) => v.tangent_f64(t),
            Curve::Rational(v) => v.tangent_f64(t),
            Curve::Nurbs(v) => v.tangent_f64(t),
        }
    }
    fn arc_length(&self) -> f64 {
        match self {
            Curve::Line(v) => v.arc_length(),
            Curve::Arc(v) => v.arc_length(),
            Curve::Ellipse(v) => v.arc_length(),
            Curve::Bezier(v) => v.arc_length(),
            Curve::Poly(v) => v.arc_length(),
            Curve::Rational(v) => v.arc_length(),
            Curve::Nurbs(v) => v.arc_length(),
        }
    }
}
