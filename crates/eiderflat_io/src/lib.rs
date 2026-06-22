pub mod dxf;
pub mod svg;
pub mod native;

pub use dxf::{import_dxf, export_dxf};
pub use svg::{import_svg, export_svg};
pub use native::{save as save_native, load as load_native, to_string as to_e2d, from_string as from_e2d};

use eiderflat_geometry::{Curve, CurveSegment, Point2d, tessellate_curve};

pub(crate) fn flatten_for_export(c: &Curve) -> Vec<Point2d> {
    let bb = c.bounding_box();
    let diag = ((bb.max.x - bb.min.x).powi(2) + (bb.max.y - bb.min.y).powi(2)).sqrt();
    let tol = (diag * 1e-3).max(1e-6);
    tessellate_curve(c, tol)
}
