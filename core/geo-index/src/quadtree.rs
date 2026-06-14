//! 自适应四叉树（Quadtree）。
//!
//! 以点集或 BBox 集构建，支持矩形范围查询。

use geo_core::types::BBox;

/// 四叉树节点。
struct QNode {
    bbox: BBox,
    /// 叶子存储的数据索引。
    items: Vec<usize>,
    /// 子节点（如果分裂），0-4 个。
    children: Vec<Box<QNode>>,
}

/// 自适应四叉树。
pub struct Quadtree {
    root: Option<QNode>,
    data_bboxes: Vec<BBox>,
    max_per_node: usize,
    max_depth: usize,
}

impl Quadtree {
    pub fn new() -> Self {
        Self {
            root: None,
            data_bboxes: Vec::new(),
            max_per_node: 8,
            max_depth: 12,
        }
    }

    pub fn with_max_per_node(mut self, m: usize) -> Self {
        self.max_per_node = m.max(2);
        self
    }

    pub fn with_max_depth(mut self, d: usize) -> Self {
        self.max_depth = d;
        self
    }

    /// 批量加载 BBox 列表。
    pub fn load(&mut self, bboxes: Vec<BBox>) {
        self.data_bboxes = bboxes;
        if self.data_bboxes.is_empty() {
            self.root = None;
            return;
        }

        let total_bbox = total_extent(&self.data_bboxes);
        let indices: Vec<usize> = (0..self.data_bboxes.len()).collect();
        self.root = Some(self.build(total_bbox, &indices, 0));
    }

    fn build(&self, bbox: BBox, indices: &[usize], depth: usize) -> QNode {
        if indices.len() <= self.max_per_node || depth >= self.max_depth {
            return QNode {
                bbox,
                items: indices.to_vec(),
                children: Vec::new(),
            };
        }

        let cx = bbox.center_x();
        let cy = bbox.center_y();

        let nw = BBox::new(bbox.min_x, cy, cx, bbox.max_y);
        let ne = BBox::new(cx, cy, bbox.max_x, bbox.max_y);
        let sw = BBox::new(bbox.min_x, bbox.min_y, cx, cy);
        let se = BBox::new(cx, bbox.min_y, bbox.max_x, cy);

        let mut nw_items = Vec::new();
        let mut ne_items = Vec::new();
        let mut sw_items = Vec::new();
        let mut se_items = Vec::new();

        for &i in indices {
            let b = &self.data_bboxes[i];
            if b.intersects(&nw) { nw_items.push(i); }
            if b.intersects(&ne) { ne_items.push(i); }
            if b.intersects(&sw) { sw_items.push(i); }
            if b.intersects(&se) { se_items.push(i); }
        }

        let mut children: Vec<Box<QNode>> = Vec::new();
        for (items, quad_bbox) in [
            (nw_items, nw),
            (ne_items, ne),
            (sw_items, sw),
            (se_items, se),
        ] {
            if !items.is_empty() && items.len() < indices.len() {
                children.push(Box::new(self.build(quad_bbox, &items, depth + 1)));
            }
        }

        if children.is_empty() {
            QNode {
                bbox,
                items: indices.to_vec(),
                children: Vec::new(),
            }
        } else {
            QNode {
                bbox,
                items: Vec::new(),
                children,
            }
        }
    }

    /// 查询与给定 BBox 相交的数据索引。
    pub fn query(&self, query_bbox: &BBox) -> Vec<usize> {
        let mut results = Vec::new();
        if let Some(ref root) = self.root {
            self.query_node(root, query_bbox, &mut results);
        }
        results.sort();
        results.dedup();
        results
    }

    fn query_node(&self, node: &QNode, query_bbox: &BBox, results: &mut Vec<usize>) {
        if !node.bbox.intersects(query_bbox) {
            return;
        }
        if node.children.is_empty() {
            results.extend(&node.items);
        } else {
            for child in &node.children {
                self.query_node(child, query_bbox, results);
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
}

impl Default for Quadtree {
    fn default() -> Self {
        Self::new()
    }
}

fn total_extent(bboxes: &[BBox]) -> BBox {
    if bboxes.is_empty() {
        return BBox::default();
    }
    let mut u = bboxes[0];
    for b in &bboxes[1..] {
        u = u.union(b);
    }
    u
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadtree_single() {
        let bboxes = vec![BBox::new(0.0, 0.0, 10.0, 10.0)];
        let mut qt = Quadtree::new();
        qt.load(bboxes);
        assert_eq!(qt.len(), 1);
        let r = qt.query(&BBox::new(5.0, 5.0, 6.0, 6.0));
        assert_eq!(r, vec![0]);
    }

    #[test]
    fn test_quadtree_miss() {
        let bboxes = vec![
            BBox::new(0.0, 0.0, 10.0, 10.0),
            BBox::new(20.0, 20.0, 30.0, 30.0),
        ];
        let mut qt = Quadtree::new();
        qt.load(bboxes);
        let r = qt.query(&BBox::new(50.0, 50.0, 60.0, 60.0));
        assert!(r.is_empty());
    }

    #[test]
    fn test_quadtree_grid() {
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
        let mut qt = Quadtree::new().with_max_per_node(4);
        qt.load(bboxes);
        assert_eq!(qt.len(), 100);
        let hits = qt.query(&BBox::new(5.0, 5.0, 25.0, 25.0));
        assert!(hits.len() >= 4);
    }
}
