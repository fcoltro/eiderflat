use super::AppState;
use crate::tools::Tool;
use eiderflat_cad::pick_at;
use eiderflat_document::EntityId;
use eiderflat_geometry::Point2d;

impl AppState {
    pub(crate) fn handle_modify_click(&mut self, p: &Point2d) -> bool {
        use eiderflat_cad::edit;
        let px = p.x;
        let py = p.y;
        let tol = self.view.pixel_world_size() * 6.0;
        let pick = |s: &Self| pick_at(&s.document, px, py, tol).filter(|&id| id != s.origin_id);

        match self.tool.clone() {
            Tool::Trim => {
                if let Some(id) = pick(self) {
                    self.history.snapshot(&self.document);
                    let cutters: Vec<EntityId> = self
                        .document
                        .iter()
                        .map(|e| e.id)
                        .filter(|&i| i != id && i != self.origin_id)
                        .collect();
                    edit::trim(&mut self.document, id, &cutters, px, py);
                    self.selection.clear();
                }
                true
            }
            Tool::Hatch => {
                self.hatch_at_point(px, py);
                true
            }
            Tool::Extend => {
                if let Some(id) = pick(self) {
                    let boundaries: Vec<EntityId> = self
                        .document
                        .iter()
                        .map(|e| e.id)
                        .filter(|&i| i != id && i != self.origin_id)
                        .collect();
                    self.history.snapshot(&self.document);
                    if !edit::extend(&mut self.document, id, &boundaries, px, py) {
                        self.history.discard_last();
                    }
                }
                true
            }
            Tool::Offset { dist, source } => {
                match source {
                    None => {
                        if let Some(id) = pick(self) {
                            self.tool = Tool::Offset {
                                dist,
                                source: Some(id),
                            };
                        }
                    }
                    Some(src) => {
                        if let Some(c) = self.document.get(src).and_then(|e| e.as_curve()).cloned()
                        {
                            let plus = eiderflat_geometry::offset_curve(&c, dist.abs());
                            let minus = eiderflat_geometry::offset_curve(&c, -dist.abs());
                            let dp = eiderflat_geometry::point_to_curve_distance(&plus, px, py);
                            let dm = eiderflat_geometry::point_to_curve_distance(&minus, px, py);
                            let signed = if dp <= dm { dist.abs() } else { -dist.abs() };
                            self.history.snapshot(&self.document);
                            edit::offset(&mut self.document, &[src], signed);
                        }
                        self.tool = Tool::Offset { dist, source: None };
                    }
                }
                true
            }
            Tool::Fillet { radius, first } => {
                if let Some(id) = pick(self) {
                    match first {
                        None => {
                            self.tool = Tool::Fillet {
                                radius,
                                first: Some(id),
                            }
                        }
                        Some(a) => {
                            if a != id {
                                self.history.snapshot(&self.document);
                                edit::fillet(&mut self.document, a, id, radius, px, py);
                            }
                            self.tool = Tool::Fillet {
                                radius,
                                first: None,
                            };
                        }
                    }
                }
                true
            }
            Tool::Chamfer { dist, first } => {
                if let Some(id) = pick(self) {
                    match first {
                        None => {
                            self.tool = Tool::Chamfer {
                                dist,
                                first: Some(id),
                            }
                        }
                        Some(a) => {
                            if a != id {
                                self.history.snapshot(&self.document);
                                edit::chamfer(&mut self.document, a, id, dist, dist);
                            }
                            self.tool = Tool::Chamfer { dist, first: None };
                        }
                    }
                }
                true
            }
            Tool::CircleTtr { radius, first } => {
                if let Some(id) = pick(self) {
                    match first {
                        None => {
                            self.tool = Tool::CircleTtr {
                                radius,
                                first: Some(id),
                            }
                        }
                        Some(a) => {
                            if a != id {
                                self.add_tangent_circle_ttr(a, id, radius, *p);
                            }
                            self.tool = Tool::CircleTtr {
                                radius,
                                first: None,
                            };
                        }
                    }
                }
                true
            }
            Tool::CircleTtt { mut picks } => {
                if let Some(id) = pick(self)
                    && !picks.contains(&id)
                {
                    picks.push(id);
                    if picks.len() == 3 {
                        self.add_tangent_circle_ttt([picks[0], picks[1], picks[2]], *p);
                        self.tool = Tool::CircleTtt { picks: Vec::new() };
                    } else {
                        self.tool = Tool::CircleTtt { picks };
                    }
                }
                true
            }
            Tool::Stretch { c1, c2, base, ids } => {
                match (c1, c2, base) {
                    (None, _, _) => {
                        let ids = if self.selection.is_empty() {
                            self.document
                                .iter()
                                .map(|e| e.id)
                                .filter(|&i| i != self.origin_id)
                                .collect()
                        } else {
                            self.selection.clone()
                        };
                        self.tool = Tool::Stretch {
                            c1: Some(*p),
                            c2: None,
                            base: None,
                            ids,
                        };
                    }
                    (Some(a), None, _) => {
                        self.tool = Tool::Stretch {
                            c1: Some(a),
                            c2: Some(*p),
                            base: None,
                            ids,
                        }
                    }
                    (Some(a), Some(b), None) => {
                        self.tool = Tool::Stretch {
                            c1: Some(a),
                            c2: Some(b),
                            base: Some(*p),
                            ids,
                        }
                    }
                    (Some(a), Some(b), Some(bp)) => {
                        let (ax, ay) = a.to_f64();
                        let (bx, by) = b.to_f64();
                        let window = (ax.min(bx), ay.min(by), ax.max(bx), ay.max(by));
                        let dx = px - bp.x;
                        let dy = py - bp.y;
                        self.history.snapshot(&self.document);
                        edit::stretch(&mut self.document, &ids, window, dx, dy);
                        self.tool = Tool::Stretch {
                            c1: None,
                            c2: None,
                            base: None,
                            ids: vec![],
                        };
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Circle of `radius` tangent to entities `a` and `b`, nearest the pick.
    fn add_tangent_circle_ttr(&mut self, a: EntityId, b: EntityId, radius: f64, near: Point2d) {
        let (Some(c1), Some(c2)) = (
            self.document.get(a).and_then(|e| e.as_curve()).cloned(),
            self.document.get(b).and_then(|e| e.as_curve()).cloned(),
        ) else {
            return;
        };
        if let Some((center, r)) = eiderflat_geometry::tangent_circle_ttr(&c1, &c2, radius, near) {
            self.create_full_circle(center, r);
        }
    }

    /// Circle tangent to three entities, nearest the final pick.
    fn add_tangent_circle_ttt(&mut self, ids: [EntityId; 3], near: Point2d) {
        let curves: Vec<_> = ids
            .iter()
            .filter_map(|&id| self.document.get(id).and_then(|e| e.as_curve()).cloned())
            .collect();
        if curves.len() != 3 {
            return;
        }
        if let Some((center, r)) =
            eiderflat_geometry::tangent_circle_ttt(&curves[0], &curves[1], &curves[2], near)
        {
            self.create_full_circle(center, r);
        }
    }

    /// Add a full circle as an entity, applying the new-object line defaults.
    fn create_full_circle(&mut self, center: Point2d, r: f64) {
        if r <= 1e-9 {
            return;
        }
        let arc = eiderflat_geometry::CircularArc::new(center, r, 0.0, std::f64::consts::TAU);
        self.apply_tool_event(crate::tools::ToolEvent::Create(vec![
            eiderflat_document::EntityKind::Curve(eiderflat_geometry::Curve::Arc(arc)),
        ]));
    }

    pub fn trim_extend_preview(&self) -> Option<TrimExtendPreview> {
        use eiderflat_cad::edit;
        let (px, py) = self.cursor_world;
        let tol = self.view.pixel_world_size() * 6.0;
        let id = pick_at(&self.document, px, py, tol)?;
        match self.tool {
            Tool::Trim => {
                let cutters: Vec<EntityId> = self
                    .document
                    .iter()
                    .map(|e| e.id)
                    .filter(|&i| i != id)
                    .collect();
                edit::trim_preview(&self.document, id, &cutters, px, py)
                    .map(TrimExtendPreview::Remove)
            }
            Tool::Extend => {
                let boundaries: Vec<EntityId> = self
                    .document
                    .iter()
                    .map(|e| e.id)
                    .filter(|&i| i != id)
                    .collect();
                edit::extend_preview(&self.document, id, &boundaries, px, py)
                    .map(TrimExtendPreview::Extension)
            }
            _ => None,
        }
    }
}

pub enum TrimExtendPreview {
    Remove(eiderflat_geometry::Curve),
    Extension(eiderflat_geometry::Curve),
}
