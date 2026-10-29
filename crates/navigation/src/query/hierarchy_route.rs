//! Runtime routing through the precomputed portal hierarchy.
use super::*;
use crate::hierarchy::PortalGraph;
use std::sync::Arc;

impl NavigationMap {
    pub(super) fn hierarchy_path(
        &mut self,
        start: Vec3,
        end: Vec3,
        plan: &mut NavigationPlan,
    ) -> Result<NavigationPath, String> {
        let graph = match &self.hierarchy {
            Some(graph) => Arc::clone(graph),
            None => {
                let graph = Arc::new(PortalGraph::read_artifact(&self.dir)?);
                self.hierarchy = Some(Arc::clone(&graph));
                graph
            }
        };
        let filter = filter();
        let (start_ref, start_pos) = self.nearest(start, &filter)?;
        let (end_ref, end_pos) = self.nearest(end, &filter)?;
        let (start_cluster, mut start_tile, start_base) =
            self.hierarchy_cluster(&graph, start_ref)?;
        let route = if plan.target == Some(end) && plan.expected_cluster == Some(start_cluster) {
            plan.remaining.clone()
        } else {
            let (end_cluster, mut end_tile, end_base) = self.hierarchy_cluster(&graph, end_ref)?;
            let starts = portal_connectors(
                &graph,
                start_cluster,
                &mut start_tile,
                start_base,
                start_ref,
                start_pos,
                false,
            )?;
            let goals = portal_connectors(
                &graph,
                end_cluster,
                &mut end_tile,
                end_base,
                end_ref,
                end_pos,
                true,
            )?;
            graph
                .route(&starts, &goals, Vec3::from_array(end_pos))?
                .ok_or("unreachable target (hierarchy disconnected)")?
        };
        let Some(crossing) = route.windows(2).position(|pair| {
            graph.nodes[pair[0] as usize].cluster != graph.nodes[pair[1] as usize].cluster
        }) else {
            let leg = self.query_corridor(start, end)?;
            let points = self.pull_corners(&leg.start, &leg.end, &leg.polygons)?;
            return Ok(NavigationPath {
                points,
                complete: leg.complete,
                collision_drop: None,
            });
        };
        let node = &graph.nodes[route[crossing + 1] as usize];
        let (destination, destination_ref) = self.portal_polygon_center(node)?;
        let (source_ref, source_pos) = self.nearest(start, &filter)?;
        let mut destination_pos = [0.0; 3];
        unsafe {
            status(
                (*self.query).closestPointOnPoly(
                    destination_ref,
                    destination.to_array().as_ptr(),
                    destination_pos.as_mut_ptr(),
                    std::ptr::null_mut(),
                ),
                "hierarchy destination polygon",
            )?;
        }
        let leg = self.query_references(
            source_ref,
            source_pos,
            destination_ref,
            destination_pos,
            &filter,
        )?;
        let points = self.pull_corners(&leg.start, &leg.end, &leg.polygons)?;
        plan.target = Some(end);
        if leg.complete {
            plan.remaining = route[crossing + 1..].to_vec();
            plan.expected_cluster = Some(node.cluster);
        } else {
            plan.remaining = route;
            plan.expected_cluster = Some(start_cluster);
        }
        Ok(NavigationPath {
            points,
            complete: false,
            collision_drop: None,
        })
    }

    fn hierarchy_cluster(
        &self,
        graph: &PortalGraph,
        reference: dtPolyRef,
    ) -> Result<(u32, NavigationMap, dtPolyRef), String> {
        let mut tile = std::ptr::null();
        let mut poly = std::ptr::null();
        unsafe {
            status(
                (*self.mesh).getTileAndPolyByRef(reference, &mut tile, &mut poly),
                "hierarchy endpoint polygon",
            )?;
        }
        let tile_slot = ((reference >> DT_POLY_BITS) & ((1u64 << DT_TILE_BITS) - 1)) as usize;
        let polygon = (reference & ((1u64 << DT_POLY_BITS) - 1)) as usize;
        let mut roots = Vec::new();
        let groups =
            unsafe { super::hierarchy_bake::build_tile_clusters(tile, tile_slot, &mut roots)? };
        let local_cluster = *groups
            .get(polygon)
            .ok_or("invalid hierarchy endpoint polygon")?;
        let root = *roots
            .get(local_cluster as usize)
            .ok_or("endpoint polygon is not walkable")?;
        let cluster = graph
            .clusters
            .iter()
            .position(|candidate| *candidate == root)
            .ok_or("hierarchy endpoint cluster missing")? as u32;
        let raw = unsafe { std::slice::from_raw_parts((*tile).data, (*tile).dataSize as usize) };
        let mut local = NavigationMap::open(&self.dir)?;
        local.add_packet(raw)?;
        let header = unsafe { &*(*tile).header };
        let local_tile = unsafe { (*local.mesh).getTileAt(header.x, header.y, header.layer) };
        if local_tile.is_null() {
            return Err("hierarchy local tile missing".into());
        }
        let base = unsafe { (*local.mesh).getPolyRefBase(local_tile) };
        Ok((cluster, local, base))
    }

    fn portal_polygon_center(
        &mut self,
        node: &crate::hierarchy::PortalNode,
    ) -> Result<(Vec3, dtPolyRef), String> {
        let point = Vec3::from_array(node.position);
        self.load_around(point, point)?;
        let key = node.polygon;
        let tile = unsafe { (*self.mesh).getTileAt(key.tile_x, key.tile_z, key.layer) };
        if tile.is_null() {
            return Err("hierarchy portal tile missing".into());
        }
        let header = unsafe { &*(*tile).header };
        if key.polygon >= header.polyCount as u32 {
            return Err("hierarchy portal polygon missing".into());
        }
        let polygon = unsafe { &*(*tile).polys.add(key.polygon as usize) };
        let vertices =
            unsafe { std::slice::from_raw_parts((*tile).verts, header.vertCount as usize * 3) };
        if polygon.vertCount == 0 {
            return Err("empty hierarchy portal polygon".into());
        }
        let mut center = Vec3::ZERO;
        for vertex in polygon.verts.iter().take(polygon.vertCount as usize) {
            let i = *vertex as usize * 3;
            center += Vec3::from_slice(&vertices[i..i + 3]);
        }
        center /= f32::from(polygon.vertCount);
        // Stop just inside the planned destination polygon. Its centroid can be
        // far from the portal and make a fresh abstract search choose the reverse.
        let reference = unsafe { (*self.mesh).getPolyRefBase(tile) } | u64::from(key.polygon);
        Ok((point.lerp(center, 0.1), reference))
    }
}

fn portal_connectors(
    graph: &PortalGraph,
    cluster: u32,
    tile: &mut NavigationMap,
    base: dtPolyRef,
    endpoint_ref: dtPolyRef,
    endpoint: [f32; 3],
    reverse: bool,
) -> Result<Vec<(u32, f32)>, String> {
    let mut connectors = Vec::new();
    let endpoint_poly = endpoint_ref & ((1u64 << DT_POLY_BITS) - 1);
    for (index, node) in graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.cluster == cluster)
    {
        let portal_ref = base | u64::from(node.polygon.polygon);
        let endpoint_ref = base | endpoint_poly;
        let (start_ref, end_ref, start, end) = if reverse {
            (portal_ref, endpoint_ref, node.position, endpoint)
        } else {
            (endpoint_ref, portal_ref, endpoint, node.position)
        };
        let cost = tile.local_portal_cost(start_ref, end_ref, start, end)?;
        connectors.push((index as u32, cost));
    }
    Ok(connectors)
}
