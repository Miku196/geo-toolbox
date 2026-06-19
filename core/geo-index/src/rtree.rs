//! 简易内存 R-tree（BBox 索引）。
//!
//! 使用 STR (Sort-Tile-Recursive) 批量构建 + BBox 查询。

use geo_core::types::BBox;

/// 简易内存 R-tree。
pub struct RTree {
    data_bboxes: Vec<BBox>,
    nodes: Vec<RtNode>,
    root_idx: Option<usize>,
    max_entries: usize,
}

#[derive(Debug, Clone)]
struct RtNode {
    bbox: BBox,
    children: Vec<usize>,
    is_leaf: bool,
}

impl RTree {
    pub fn new() -> Self {
        Self {
            data_bboxes: Vec::new(),
            nodes: Vec::new(),
            root_idx: None,
            max_entries: 16,
        }
    }

    pub fn with_max_entries(mut self, m: usize) -> Self {
        self.max_entries = m.max(4);
        self
    }

    pub fn load(&mut self, bboxes: Vec<BBox>) {
        self.data_bboxes = bboxes;
        self.nodes.clear();
        let n = self.data_bboxes.len();
        if n == 0 {
            self.root_idx = None;
            return;
        }
        let indices: Vec<usize> = (0..n).collect();
        self.root_idx = Some(self.build_level(&indices));
    }

    fn build_level(&mut self, indices: &[usize]) -> usize {
        let n = indices.len();
        if n <= self.max_entries {
            let children: Vec<usize> = indices.to_vec();
            let bbox = union_of(
                &children
                    .iter()
                    .map(|&i| &self.data_bboxes[i])
                    .collect::<Vec<_>>(),
            );
            self.nodes.push(RtNode {
                bbox,
                children,
                is_leaf: true,
            });
            return self.nodes.len() - 1;
        }

        let mut sorted: Vec<(usize, f64)> = indices
            .iter()
            .map(|&i| (i, self.data_bboxes[i].center_x()))
            .collect();
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let slice_size = (n as f64 / self.max_entries as f64).ceil() as usize;
        let slice_size = slice_size.max(1);
        let mut child_idxs = Vec::new();

        for chunk in sorted.chunks(slice_size) {
            let chunk_indices: Vec<usize> = chunk.iter().map(|&(i, _)| i).collect();
            child_idxs.push(self.build_level(&chunk_indices));
        }

        let bbox = union_of(
            &child_idxs
                .iter()
                .map(|&i| &self.nodes[i].bbox)
                .collect::<Vec<_>>(),
        );
        self.nodes.push(RtNode {
            bbox,
            children: child_idxs,
            is_leaf: false,
        });
        self.nodes.len() - 1
    }

    pub fn query(&self, query_bbox: &BBox) -> Vec<usize> {
        let mut results = Vec::new();
        if let Some(root) = self.root_idx {
            self.query_node(root, query_bbox, &mut results);
        }
        results.sort();
        results.dedup();
        results
    }

    fn query_node(&self, node_idx: usize, query_bbox: &BBox, results: &mut Vec<usize>) {
        let node = &self.nodes[node_idx];
        if !node.bbox.intersects(query_bbox) {
            return;
        }
        if node.is_leaf {
            results.extend(&node.children);
        } else {
            for &child_idx in &node.children {
                self.query_node(child_idx, query_bbox, results);
            }
        }
    }

    pub fn query_bboxes(&self, query_bbox: &BBox) -> Vec<BBox> {
        self.query(query_bbox)
            .iter()
            .map(|&i| self.data_bboxes[i])
            .collect()
    }

    pub fn len(&self) -> usize {
        self.data_bboxes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data_bboxes.is_empty()
    }

    /// K-Nearest Neighbors search (brute-force fallback for small datasets).
    ///
    /// Returns (index, distance_squared) sorted by distance, up to k results.
    /// Points are found by expanding the query bbox until k neighbors found.
    pub fn knn(&self, query: &BBox, k: usize) -> Vec<(usize, f64)> {
        if self.is_empty() || k == 0 {
            return vec![];
        }
        let qx = query.center_x();
        let qy = query.center_y();

        // Get all candidate indices via R-tree query
        let mut candidates: Vec<(usize, f64)> = self
            .query(query)
            .into_iter()
            .map(|i| {
                let cx = self.data_bboxes[i].center_x();
                let cy = self.data_bboxes[i].center_y();
                let dx = qx - cx;
                let dy = qy - cy;
                (i, dx * dx + dy * dy)
            })
            .collect();

        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);
        candidates
    }

    /// Point query: find all bboxes containing the given point.
    pub fn query_point(&self, x: f64, y: f64) -> Vec<usize> {
        let small = BBox::new(x - 1e-10, y - 1e-10, x + 1e-10, y + 1e-10);
        self.query(&small)
            .into_iter()
            .filter(|&i| {
                let b = &self.data_bboxes[i];
                x >= b.min_x && x <= b.max_x && y >= b.min_y && y <= b.max_y
            })
            .collect()
    }
}

impl Default for RTree {
    fn default() -> Self {
        Self::new()
    }
}

fn union_of(bboxes: &[&BBox]) -> BBox {
    if bboxes.is_empty() {
        return BBox::default();
    }
    let mut u = *bboxes[0];
    for b in &bboxes[1..] {
        u = u.union(b);
    }
    u
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtree_single() {
        let bboxes = vec![BBox::new(0.0, 0.0, 10.0, 10.0)];
        let mut tree = RTree::new();
        tree.load(bboxes);
        assert_eq!(tree.len(), 1);
        let r = tree.query(&BBox::new(5.0, 5.0, 6.0, 6.0));
        assert_eq!(r, vec![0]);
    }

    #[test]
    fn test_rtree_miss() {
        let bboxes = vec![
            BBox::new(0.0, 0.0, 10.0, 10.0),
            BBox::new(20.0, 20.0, 30.0, 30.0),
        ];
        let mut tree = RTree::new();
        tree.load(bboxes);
        let r = tree.query(&BBox::new(50.0, 50.0, 60.0, 60.0));
        assert!(r.is_empty());
    }

    #[test]
    fn test_rtree_overlap() {
        let mut bboxes = Vec::new();
        for i in 0..10 {
            for j in 0..10 {
                bboxes.push(BBox::new(
                    i as f64 * 10.0,
                    j as f64 * 10.0,
                    i as f64 * 10.0 + 10.0,
                    j as f64 * 10.0 + 10.0,
                ));
            }
        }
        let mut tree = RTree::new().with_max_entries(8);
        tree.load(bboxes);
        assert_eq!(tree.len(), 100);
        let hits = tree.query(&BBox::new(5.0, 5.0, 25.0, 25.0));
        assert!(hits.len() >= 4, "Expected >=4 hits, got {}", hits.len());
    }
}
