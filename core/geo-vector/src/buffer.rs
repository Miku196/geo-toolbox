use geo::algorithm::{Area, BooleanOps, BoundingRect, ConvexHull};
use geo_types::{Coord, LineString, MultiPolygon, Polygon};

/// 缓冲区模式选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferMode {
    /// O(1) 轴对齐 BBox 矩形外扩（最快，忽略形状）。
    Bbox,
    /// O(n) 凸壳近似（适合凸多边形，凹形会被填充）。
    ConvexHull { quadrant_segments: u8 },
    /// O(n²) 精确偏移：逐边挤推平行四边形 + 顶点圆弧 + BooleanOps Union。
    /// 参考 Shapely / JTS buffer 算法。
    Precise { quadrant_segments: u8 },
}

/// 最大顶点数限制（防止恶意输入导致 OOM）。
pub const MAX_BUFFER_VERTICES: usize = 500_000;

/// 对多边形进行缓冲区分析。
///
/// # 参数
/// - `poly`: 输入多边形
/// - `distance`: 缓冲距离（正=外扩，负=内缩，0=原样返回）
/// - `mode`: 缓冲区模式选择
pub fn buffer(poly: &Polygon<f64>, distance: f64, mode: BufferMode) -> MultiPolygon<f64> {
    let n = poly.exterior().0.len();
    if !(3..=MAX_BUFFER_VERTICES).contains(&n) {
        return MultiPolygon::new(vec![poly.clone()]);
    }
    if distance == 0.0 {
        return MultiPolygon::new(vec![poly.clone()]);
    }

    match mode {
        BufferMode::Bbox => {
            if distance > 0.0 {
                bbox_buffer_outer(poly, distance)
            } else {
                bbox_buffer_inner(poly, -distance)
            }
        }
        BufferMode::ConvexHull { quadrant_segments } => {
            if distance > 0.0 {
                convexhull_buffer_outer(poly, distance, quadrant_segments.clamp(4, 32) as usize)
            } else {
                bbox_buffer_inner(poly, -distance)
            }
        }
        BufferMode::Precise { quadrant_segments } => {
            if distance > 0.0 {
                precise_buffer_outer(poly, distance, quadrant_segments.clamp(4, 32) as usize)
            } else {
                bbox_buffer_inner(poly, -distance)
            }
        }
    }
}

// ═══════════════════════ BBox 模式 ═══════════════════════

pub(crate) fn bbox_buffer_outer(poly: &Polygon<f64>, dist: f64) -> MultiPolygon<f64> {
    let bbox = match poly.bounding_rect() {
        Some(r) => r,
        None => return MultiPolygon::new(vec![poly.clone()]),
    };
    let min = bbox.min();
    let max = bbox.max();
    MultiPolygon::new(vec![Polygon::new(
        LineString::new(vec![
            Coord {
                x: min.x - dist,
                y: min.y - dist,
            },
            Coord {
                x: max.x + dist,
                y: min.y - dist,
            },
            Coord {
                x: max.x + dist,
                y: max.y + dist,
            },
            Coord {
                x: min.x - dist,
                y: max.y + dist,
            },
            Coord {
                x: min.x - dist,
                y: min.y - dist,
            },
        ]),
        vec![],
    )])
}

pub(crate) fn bbox_buffer_inner(poly: &Polygon<f64>, dist: f64) -> MultiPolygon<f64> {
    let bbox = match poly.bounding_rect() {
        Some(r) => r,
        None => return MultiPolygon::new(vec![poly.clone()]),
    };
    let min = bbox.min();
    let max = bbox.max();
    let cx = (min.x + max.x) / 2.0;
    let cy = (min.y + max.y) / 2.0;
    let hw = (max.x - min.x) / 2.0 - dist;
    let hh = (max.y - min.y) / 2.0 - dist;
    if hw <= 0.0 || hh <= 0.0 {
        return MultiPolygon::new(vec![]);
    }
    MultiPolygon::new(vec![Polygon::new(
        LineString::new(vec![
            Coord {
                x: cx - hw,
                y: cy - hh,
            },
            Coord {
                x: cx + hw,
                y: cy - hh,
            },
            Coord {
                x: cx + hw,
                y: cy + hh,
            },
            Coord {
                x: cx - hw,
                y: cy + hh,
            },
            Coord {
                x: cx - hw,
                y: cy - hh,
            },
        ]),
        vec![],
    )])
}

// ═══════════════════════ ConvexHull 模式 ═══════════════════════

fn convexhull_buffer_outer(poly: &Polygon<f64>, dist: f64, segments: usize) -> MultiPolygon<f64> {
    let exterior = poly.exterior();
    let coords = &exterior.0;
    if coords.len() < 3 {
        return MultiPolygon::new(vec![poly.clone()]);
    }

    let total_len: f64 = coords
        .windows(2)
        .map(|w| {
            let dx = w[1].x - w[0].x;
            let dy = w[1].y - w[0].y;
            (dx * dx + dy * dy).sqrt()
        })
        .sum();
    let step = (total_len / (coords.len() as f64 * segments as f64)).max(dist / 4.0);

    let mut offset_points: Vec<Coord<f64>> = Vec::new();
    for w in coords.windows(2) {
        let dx = w[1].x - w[0].x;
        let dy = w[1].y - w[0].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 {
            continue;
        }
        let nx = dy / len;
        let ny = -dx / len;
        let n_steps = (len / step).ceil() as usize;
        for i in 0..=n_steps {
            let t = i as f64 / n_steps as f64;
            offset_points.push(Coord {
                x: w[0].x + dx * t + nx * dist,
                y: w[0].y + dy * t + ny * dist,
            });
        }
    }

    if offset_points.len() < 3 {
        return bbox_buffer_outer(poly, dist);
    }

    let hull_poly = Polygon::new(LineString::new(offset_points), vec![]).convex_hull();
    MultiPolygon::new(vec![hull_poly])
}

// ═══════════════════════ Precise 模式 ═══════════════════════

/// 精确外扩缓冲：逐边平行四边形挤推 + 顶点圆弧 + BooleanOps Union。
#[derive(Clone, Copy)]
struct EdgeNormal {
    nx: f64,
    ny: f64,
    len: f64,
}

/// Pre-compute outward edge normals for each segment of the polygon ring.
fn compute_edge_normals(coords: &[Coord<f64>], n: usize) -> Vec<EdgeNormal> {
    let mut normals = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = coords[j].x - coords[i].x;
        let dy = coords[j].y - coords[i].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 {
            normals.push(EdgeNormal {
                nx: 0.0,
                ny: 0.0,
                len: 0.0,
            });
        } else {
            normals.push(EdgeNormal {
                nx: dy / len,
                ny: -dx / len,
                len,
            });
        }
    }
    normals
}

/// Build offset parallelograms for every edge of the polygon.
fn generate_offset_parallelograms(
    coords: &[Coord<f64>],
    n: usize,
    normals: &[EdgeNormal],
    dist: f64,
) -> Vec<Polygon<f64>> {
    let mut parts = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let nv = &normals[i];
        if nv.len < 1e-12 {
            continue;
        }
        let ib = coords[i];
        let jb = coords[j];
        let i_off = Coord {
            x: ib.x + nv.nx * dist,
            y: ib.y + nv.ny * dist,
        };
        let j_off = Coord {
            x: jb.x + nv.nx * dist,
            y: jb.y + nv.ny * dist,
        };
        parts.push(Polygon::new(
            LineString::new(vec![ib, i_off, j_off, jb, ib]),
            vec![],
        ));
    }
    parts
}

/// Insert arc polygons at convex vertices to round the offset corners.
fn build_convex_vertex_arcs(
    coords: &[Coord<f64>],
    n: usize,
    normals: &[EdgeNormal],
    dist: f64,
    segments: usize,
) -> Vec<Polygon<f64>> {
    let step_angle = std::f64::consts::FRAC_PI_2 / (segments as f64).max(1.0);
    let mut parts = Vec::with_capacity(n);
    for i in 0..n {
        let prev = (i + n - 1) % n;
        let curr = i;

        let n1 = &normals[prev];
        let n2 = &normals[curr];
        if n1.len < 1e-12 || n2.len < 1e-12 {
            continue;
        }

        let prev_pt = coords[prev];
        let curr_pt = coords[curr];
        let next_pt = coords[(curr + 1) % n];

        let dx1 = curr_pt.x - prev_pt.x;
        let dy1 = curr_pt.y - prev_pt.y;
        let dx2 = next_pt.x - curr_pt.x;
        let dy2 = next_pt.y - curr_pt.y;
        let cross = dx1 * dy2 - dy1 * dx2;

        if cross <= 0.0 {
            continue;
        }

        let prev_offset_end = Coord {
            x: curr_pt.x + n1.nx * dist,
            y: curr_pt.y + n1.ny * dist,
        };
        let curr_offset_start = Coord {
            x: curr_pt.x + n2.nx * dist,
            y: curr_pt.y + n2.ny * dist,
        };

        let angle1 = (prev_offset_end.y - curr_pt.y).atan2(prev_offset_end.x - curr_pt.x);
        let angle2 = (curr_offset_start.y - curr_pt.y).atan2(curr_offset_start.x - curr_pt.x);

        let mut sweep = angle2 - angle1;
        while sweep < -std::f64::consts::PI {
            sweep += std::f64::consts::TAU;
        }
        while sweep > std::f64::consts::PI {
            sweep -= std::f64::consts::TAU;
        }

        let arc_steps = ((sweep.abs() / step_angle).ceil() as usize).max(2);
        let arc_step = sweep / arc_steps as f64;

        let mut arc_coords: Vec<Coord<f64>> = Vec::with_capacity(arc_steps + 2);
        arc_coords.push(curr_pt);
        for k in 0..=arc_steps {
            let a = angle1 + arc_step * k as f64;
            arc_coords.push(Coord {
                x: curr_pt.x + a.cos() * dist,
                y: curr_pt.y + a.sin() * dist,
            });
        }

        parts.push(Polygon::new(LineString::new(arc_coords), vec![]));
    }
    parts
}

/// Union all geometry parts into a single MultiPolygon, falling back to BBox on failure.
fn union_geometry_parts(
    parts: Vec<Polygon<f64>>,
    poly: &Polygon<f64>,
    dist: f64,
) -> MultiPolygon<f64> {
    if parts.is_empty() {
        return bbox_buffer_outer(poly, dist);
    }
    let mut result = MultiPolygon::new(vec![parts[0].clone()]);
    for part in &parts[1..] {
        result = result.union(&MultiPolygon::new(vec![part.clone()]));
    }
    result
}

fn precise_buffer_outer(poly: &Polygon<f64>, dist: f64, segments: usize) -> MultiPolygon<f64> {
    let exterior = poly.exterior();
    let coords = &exterior.0;
    let n = coords.len() - 1;
    if n < 3 {
        return MultiPolygon::new(vec![poly.clone()]);
    }

    let edge_normals = compute_edge_normals(coords, n);
    let mut parts: Vec<Polygon<f64>> = Vec::with_capacity(n * 2);

    parts.push(poly.clone());

    let pgrams = generate_offset_parallelograms(coords, n, &edge_normals, dist);
    parts.extend(pgrams);

    let arcs = build_convex_vertex_arcs(coords, n, &edge_normals, dist, segments);
    parts.extend(arcs);

    union_geometry_parts(parts, poly, dist)
}
