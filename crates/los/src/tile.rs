//! One baked map tile: triangles in Bevy world space and the BVH over them.
//!
//! File layout (little endian): magic `LOS1`, then `u32` row, col, vertex count, triangle
//! count and node count, then the vertices (`f32` x3), the triangles (`u32` vertex
//! indices x3, in BVH leaf order) and the nodes (`f32` min x3, `f32` max x3, `u32` start,
//! `u32` count; a leaf holds `count > 0` triangles from `start`, an interior node's
//! children are nodes `start` and `start + 1`). Node 0 is the root.

use glam::Vec3;

const MAGIC: &[u8; 4] = b"LOS1";
const HEADER_SIZE: usize = 24;
const NODE_SIZE: usize = 32;
/// Triangles per BVH leaf.
const LEAF_SIZE: usize = 4;
/// SAH bins per split.
const BINS: usize = 16;
/// BVH depth bound for the traversal stack.
const MAX_DEPTH: usize = 64;
/// Below this depth the build uses median splits only, which halve the triangles: a run
/// of lopsided SAH splits cannot reach `MAX_DEPTH`.
const SAH_DEPTH: usize = 32;
/// Segment parameter margin at both ends: a triangle touching the segment's start or end
/// does not block it (a ray from a point resting on a surface).
const END_EPSILON: f32 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Node {
    min: Vec3,
    max: Vec3,
    start: u32,
    count: u32,
}

#[derive(Debug, PartialEq)]
pub struct LosTile {
    pub row: u32,
    pub col: u32,
    vertices: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    nodes: Vec<Node>,
}

impl LosTile {
    /// Builds the BVH over `triangles` (indices into `vertices`).
    pub fn build(row: u32, col: u32, vertices: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        let (nodes, order) = build_bvh(&vertices, &triangles);
        let triangles = order
            .iter()
            .map(|&index| triangles[index as usize])
            .collect();
        Self {
            row,
            col,
            vertices,
            triangles,
            nodes,
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Bytes the loaded tile holds.
    pub fn memory_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.vertices.capacity() * std::mem::size_of::<Vec3>()
            + self.triangles.capacity() * std::mem::size_of::<[u32; 3]>()
            + self.nodes.capacity() * std::mem::size_of::<Node>()
    }

    /// World bounds of the tile's triangles; `None` for an empty tile.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.nodes.first().map(|root| (root.min, root.max))
    }

    /// Whether any triangle crosses the open segment `a..b`.
    pub fn blocks(&self, a: Vec3, b: Vec3) -> bool {
        self.hit(a, b, true).is_some()
    }

    /// Segment parameter (0 at `a`, 1 at `b`) of the first triangle the segment crosses.
    pub fn first_hit(&self, a: Vec3, b: Vec3) -> Option<f32> {
        self.hit(a, b, false)
    }

    fn hit(&self, a: Vec3, b: Vec3, any: bool) -> Option<f32> {
        if self.nodes.is_empty() {
            return None;
        }
        let dir = b - a;
        // A zero component gives a huge, finite inverse: no 0 * inf NaN in the slab test.
        let safe = Vec3::select(dir.abs().cmplt(Vec3::splat(1e-30)), Vec3::splat(1e-30), dir);
        let inv = safe.recip();
        let mut best = 1.0 - END_EPSILON;
        let mut found = None;
        let mut stack = [0u32; MAX_DEPTH];
        let mut depth = 1;
        while depth > 0 {
            depth -= 1;
            let node = self.nodes[stack[depth] as usize];
            if !slab_hit(node.min, node.max, a, inv, best) {
                continue;
            }
            if node.count == 0 {
                stack[depth] = node.start;
                stack[depth + 1] = node.start + 1;
                depth += 2;
                continue;
            }
            for tri in &self.triangles[node.start as usize..(node.start + node.count) as usize] {
                let corners = tri.map(|index| self.vertices[index as usize]);
                if let Some(t) = segment_triangle(a, dir, corners, best) {
                    if any {
                        return Some(t);
                    }
                    best = t;
                    found = Some(t);
                }
            }
        }
        found
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            HEADER_SIZE
                + self.vertices.len() * 12
                + self.triangles.len() * 12
                + self.nodes.len() * NODE_SIZE,
        );
        out.extend_from_slice(MAGIC);
        for value in [
            self.row,
            self.col,
            self.vertices.len() as u32,
            self.triangles.len() as u32,
            self.nodes.len() as u32,
        ] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for vertex in &self.vertices {
            push_vec3(&mut out, *vertex);
        }
        for tri in &self.triangles {
            tri.iter()
                .for_each(|index| out.extend_from_slice(&index.to_le_bytes()));
        }
        for node in &self.nodes {
            push_vec3(&mut out, node.min);
            push_vec3(&mut out, node.max);
            out.extend_from_slice(&node.start.to_le_bytes());
            out.extend_from_slice(&node.count.to_le_bytes());
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < HEADER_SIZE || &data[..4] != MAGIC {
            return Err("not a LOS1 tile".into());
        }
        let header = |index: usize| u32_at(data, 4 + index * 4) as usize;
        let (vertex_count, tri_count, node_count) = (header(2), header(3), header(4));
        let expected = HEADER_SIZE + vertex_count * 12 + tri_count * 12 + node_count * NODE_SIZE;
        if data.len() != expected {
            return Err(format!(
                "LOS tile is {} bytes, header says {expected}",
                data.len()
            ));
        }
        let mut offset = HEADER_SIZE;
        let vertices: Vec<Vec3> = (0..vertex_count)
            .map(|index| vec3_at(data, offset + index * 12))
            .collect();
        offset += vertex_count * 12;
        let triangles: Vec<[u32; 3]> = (0..tri_count)
            .map(|index| {
                let base = offset + index * 12;
                [0, 1, 2].map(|corner| u32_at(data, base + corner * 4))
            })
            .collect();
        offset += tri_count * 12;
        let nodes: Vec<Node> = (0..node_count)
            .map(|index| {
                let base = offset + index * NODE_SIZE;
                Node {
                    min: vec3_at(data, base),
                    max: vec3_at(data, base + 12),
                    start: u32_at(data, base + 24),
                    count: u32_at(data, base + 28),
                }
            })
            .collect();
        validate(&vertices, &triangles, &nodes)?;
        Ok(Self {
            row: header(0) as u32,
            col: header(1) as u32,
            vertices,
            triangles,
            nodes,
        })
    }
}

/// Indices in range and a tree the traversal can walk (children after their parent).
fn validate(vertices: &[Vec3], triangles: &[[u32; 3]], nodes: &[Node]) -> Result<(), String> {
    if let Some(tri) = triangles
        .iter()
        .find(|tri| tri.iter().any(|&index| index as usize >= vertices.len()))
    {
        return Err(format!(
            "triangle {tri:?} indexes past {} vertices",
            vertices.len()
        ));
    }
    let mut depths = vec![0usize; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        let valid = if node.count == 0 {
            let children = node.start as usize > index && (node.start as usize + 1) < nodes.len();
            if children {
                depths[node.start as usize] = depths[index] + 1;
                depths[node.start as usize + 1] = depths[index] + 1;
            }
            children && depths[index] + 2 < MAX_DEPTH
        } else {
            (node.start + node.count) as usize <= triangles.len()
        };
        if !valid {
            return Err(format!("BVH node {index} is out of range: {node:?}"));
        }
    }
    Ok(())
}

fn push_vec3(out: &mut Vec<u8>, value: Vec3) {
    for component in value.to_array() {
        out.extend_from_slice(&component.to_le_bytes());
    }
}

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn vec3_at(data: &[u8], offset: usize) -> Vec3 {
    let f = |index: usize| f32::from_bits(u32_at(data, offset + index * 4));
    Vec3::new(f(0), f(1), f(2))
}

/// Whether the segment `origin + t * dir`, `t` in `[0, t_max]`, meets the box.
fn slab_hit(min: Vec3, max: Vec3, origin: Vec3, inv: Vec3, t_max: f32) -> bool {
    let t0 = (min - origin) * inv;
    let t1 = (max - origin) * inv;
    let near = t0.min(t1).max_element().max(0.0);
    let far = t0.max(t1).min_element().min(t_max);
    near <= far
}

/// Möller–Trumbore, both faces: the segment parameter of the crossing in
/// `(END_EPSILON, t_max)`.
fn segment_triangle(origin: Vec3, dir: Vec3, [a, b, c]: [Vec3; 3], t_max: f32) -> Option<f32> {
    let edge1 = b - a;
    let edge2 = c - a;
    let p = dir.cross(edge2);
    let det = edge1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv_det = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(edge1);
    let v = dir.dot(q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = edge2.dot(q) * inv_det;
    (t > END_EPSILON && t < t_max).then_some(t)
}

#[derive(Clone, Copy)]
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    const EMPTY: Self = Self {
        min: Vec3::splat(f32::MAX),
        max: Vec3::splat(f32::MIN),
    };

    fn grow(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn grow_point(self, point: Vec3) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    fn area(self) -> f32 {
        let extent = (self.max - self.min).max(Vec3::ZERO);
        2.0 * (extent.x * extent.y + extent.y * extent.z + extent.z * extent.x)
    }
}

/// Binned-SAH BVH: the nodes and the triangle order its leaves index.
fn build_bvh(vertices: &[Vec3], triangles: &[[u32; 3]]) -> (Vec<Node>, Vec<u32>) {
    if triangles.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let boxes: Vec<Aabb> = triangles
        .iter()
        .map(|tri| {
            tri.iter().fold(Aabb::EMPTY, |aabb, &index| {
                aabb.grow_point(vertices[index as usize])
            })
        })
        .collect();
    let centroids: Vec<Vec3> = boxes
        .iter()
        .map(|aabb| (aabb.min + aabb.max) * 0.5)
        .collect();
    let mut order: Vec<u32> = (0..triangles.len() as u32).collect();
    let mut nodes = vec![Node {
        min: Vec3::ZERO,
        max: Vec3::ZERO,
        start: 0,
        count: 0,
    }];
    let mut pending = vec![(0usize, 0usize, order.len(), 0usize)];
    while let Some((node_index, start, end, depth)) = pending.pop() {
        let range = &mut order[start..end];
        let bounds = range
            .iter()
            .fold(Aabb::EMPTY, |aabb, &tri| aabb.grow(boxes[tri as usize]));
        nodes[node_index].min = bounds.min;
        nodes[node_index].max = bounds.max;
        let split = (range.len() > LEAF_SIZE)
            .then(|| split_range(range, &boxes, &centroids, depth < SAH_DEPTH))
            .flatten();
        let Some(mid) = split else {
            nodes[node_index].start = start as u32;
            nodes[node_index].count = (end - start) as u32;
            continue;
        };
        let left = nodes.len();
        nodes[node_index].start = left as u32;
        nodes[node_index].count = 0;
        nodes.extend([nodes[node_index]; 2]);
        pending.push((left, start, start + mid, depth + 1));
        pending.push((left + 1, start + mid, end, depth + 1));
    }
    (nodes, order)
}

/// Partitions `range` and returns the left side's length: the cheapest SAH bin boundary
/// on the centroids' longest axis, or a median split when every centroid shares a bin or
/// `sah` is off.
fn split_range(range: &mut [u32], boxes: &[Aabb], centroids: &[Vec3], sah: bool) -> Option<usize> {
    let centroid_bounds = range.iter().fold(Aabb::EMPTY, |aabb, &tri| {
        aabb.grow_point(centroids[tri as usize])
    });
    let extent = centroid_bounds.max - centroid_bounds.min;
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    if !sah || extent[axis] <= 0.0 {
        return median_split(range, centroids, axis);
    }
    let bin_of = |tri: u32| {
        let offset = (centroids[tri as usize][axis] - centroid_bounds.min[axis]) / extent[axis];
        ((offset * BINS as f32) as usize).min(BINS - 1)
    };
    let mut bins = [(Aabb::EMPTY, 0usize); BINS];
    for &tri in range.iter() {
        let bin = &mut bins[bin_of(tri)];
        bin.0 = bin.0.grow(boxes[tri as usize]);
        bin.1 += 1;
    }
    let mut right_costs = [0.0f32; BINS];
    let (mut aabb, mut count) = (Aabb::EMPTY, 0);
    for split in (1..BINS).rev() {
        aabb = aabb.grow(bins[split].0);
        count += bins[split].1;
        right_costs[split] = if count == 0 {
            0.0
        } else {
            aabb.area() * count as f32
        };
    }
    let (mut aabb, mut count) = (Aabb::EMPTY, 0);
    let mut best: Option<(usize, f32)> = None;
    for split in 1..BINS {
        aabb = aabb.grow(bins[split - 1].0);
        count += bins[split - 1].1;
        if count == 0 || count == range.len() {
            continue;
        }
        let cost = aabb.area() * count as f32 + right_costs[split];
        if best.is_none_or(|(_, best_cost)| cost < best_cost) {
            best = Some((split, cost));
        }
    }
    let Some((split, _)) = best else {
        return median_split(range, centroids, axis);
    };
    let mut mid = 0;
    for index in 0..range.len() {
        if bin_of(range[index]) < split {
            range.swap(index, mid);
            mid += 1;
        }
    }
    Some(mid)
}

fn median_split(range: &mut [u32], centroids: &[Vec3], axis: usize) -> Option<usize> {
    let mid = range.len() / 2;
    range.select_nth_unstable_by(mid, |&a, &b| {
        centroids[a as usize][axis].total_cmp(&centroids[b as usize][axis])
    });
    Some(mid)
}

#[cfg(test)]
#[path = "tile_tests.rs"]
mod tests;
