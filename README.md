# eiderFLAT

**eiderFLAT** is a from-scratch **2D CAD system written in Rust** — a fast,
direct-manipulation drawing environment in the spirit of old cad softwares, with a
modern app-style interface.

The geometry kernel is built on **f64 coordinates with tolerance-based
predicates**: lines, arcs, ellipses, cubic Béziers, polycurves, and clamped-cubic
**NURBS** are first-class primitives. Intersections, offsets, distances, and
planar booleans are computed numerically, with **Shewchuk-exact orientation
predicates** keeping boolean winding robust. The viewport is the egui painter with
adaptive, zoom-aware tessellation — smooth at any zoom, exact where it matters.

## Features

**Drawing**
- Line, Polyline, Circle, Arc (3-point), Ellipse, Rectangle, Polygon (n-sided)
- **NURBS spline** (control-vertex authoring; draggable CV grips with per-vertex weights)
- Text (single- and multi-line) with a user-selectable on-canvas font (TTF/OTF)

**Modifying**
- Move, Copy, Rotate, Scale, Mirror, Stretch
- Offset (segment-mitred for polylines/polygons; exact for NURBS)
- Trim, Extend (span-aware, spline-preserving), Fillet, Chamfer
- Explode / Join, and **Hatch** (region-based solid fill with islands)
- CAD-style **grips** on every selected entity (drag to reshape; type exact values)
- Contextual corner fillet/chamfer dots, bounding-box transform handles

**Workspace**
- Layers (colour, show/hide, rename, current-layer), editable **Properties** inspector
- Object snapping (Endpoint/Midpoint/Center/Quadrant/Intersection/Perpendicular/
  Tangent/Nearest/Node), grid + grid snap, polar/angle guides, dynamic input HUD
- Window/crossing marquee, hover highlight, ghost previews for transforms
- **Ctrl+K command palette** and an always-visible command line
- Drawing units (mm/cm/m/km/in/ft/unitless) that bound the zoom range
- Boolean region kernel (Greiner–Hormann clipping, robust orientation)

**Interoperability**
- Native **`.e2d`** format (lossless), **DXF** (ASCII) and **SVG** import/export

## Commands

Type a word in the command line (or use the toolbars / menus). Common aliases:

| Draw | | Modify | | Other | |
|------|--|--------|--|-------|--|
| `LINE` / `L` | Line | `MOVE` / `M` | Move | `SELECT` / `SE` | Select |
| `POLYLINE` / `PL` | Polyline | `COPY` / `CO` | Copy | `ERASE` / `E` / `DEL` | Delete |
| `CIRCLE` / `C` | Circle | `ROTATE` / `RO` | Rotate | `EXPLODE` / `X` | Explode |
| `ARC` / `A` | Arc (3-pt) | `SCALE` / `SC` | Scale | `JOIN` / `J` | Join |
| `ELLIPSE` / `EL` | Ellipse | `MIRROR` / `MI` | Mirror | `HATCH` / `H` | Hatch |
| `RECTANGLE` / `REC` | Rectangle | `OFFSET` / `O` | Offset | `UNDO` / `U` | Undo |
| `POLYGON` / `POL` | Polygon | `TRIM` / `TR` | Trim | `REDO` | Redo |
| `SPLINE` / `SPL` | NURBS spline | `EXTEND` / `EX` | Extend | `ALL` | Select all |
| `TEXT` / `T` / `MTEXT` | Text | `FILLET` / `F` | Fillet | `ZOOM` / `Z` | Zoom |
| | | `CHAMFER` / `CHA` | Chamfer | `LAYER` / `LA` | Layers |
| | | `STRETCH` / `S` | Stretch | | |

Coordinate entry supports `x,y` (absolute), `@dx,dy` (relative), `d<a` (polar
absolute) and `@d<a` (polar relative, degrees).

## Build & run

Plain Cargo, no special toolchain:

```sh
cargo build --workspace
cargo test  --workspace

cargo run -p eiderflat_app          # launch the interactive CAD window
cargo run -p eiderflat_app -- demo  # headless geometry-kernel demo
```

## Workspace layout

| Crate | Responsibility |
|-------|----------------|
| `eiderflat_geometry` | Curve primitives (line, arc, ellipse, cubic, polycurve, NURBS), transforms, ops (intersect/distance/curvature/offset/split) |
| `eiderflat_spatial` | Adaptive quadtree + Morton-code spatial index |
| `eiderflat_boolean` | Planar region boolean ops (union/intersection/difference/xor) with robust winding |
| `eiderflat_document` | Document / layer / entity / block model |
| `eiderflat_cad` | Snapping, selection, grips, draw + edit (trim/extend/fillet/chamfer/offset/hatch/…) |
| `eiderflat_io` | DXF, SVG, and native `.e2d` import/export |
| `eiderflat_ui` | Headless app state + egui view (toolbars, canvas, panels, command palette) |
| `apps/eiderflat_app` | eframe GUI host + headless kernel demo |

> The `eiderflat_*` crate prefix and the `.e2d` format magic are internal identifiers
> kept for stability; the product is named **eiderFLAT**.

## License

**eiderFLAT is free software, licensed under the GNU General Public License v3.0 or
later** (`GPL-3.0-or-later`) — see [LICENSE](LICENSE). You may use, study, modify,
and redistribute it under those terms; derivative works must remain GPL-licensed
and share their source.