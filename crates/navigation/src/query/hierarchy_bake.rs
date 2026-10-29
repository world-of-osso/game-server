//! Offline extraction of connected tile regions and their Detour portal costs.
use super::*;
use crate::hierarchy::{PolygonKey, PortalEdge, PortalGraph, PortalNode};
use std::collections::HashMap;

const NO_CLUSTER: u32 = u32::MAX;

struct RawPortal {
    from: dtPolyRef,
    to: dtPolyRef,
}

impl NavigationMap {
    /// Build and persist a graph using every baked tile, never runtime source geometry.
    pub fn bake_portal_graph(&mut self) -> Result<(), String> {
        self.load_graph_tiles()?;
        let graph = self.build_portal_graph()?;
        graph.write_artifact(&self.dir)
    }

    pub(super) fn load_graph_tiles(&mut self) -> Result<(), String> {
        let mut tiles = Vec::new();
        for entry in std::fs::read_dir(&self.dir)
            .map_err(|error| format!("{}: {error}", self.dir.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().is_none_or(|extension| extension != "nav") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or("invalid navmesh filename")?;
            let (row, col) = name
                .split_once('_')
                .ok_or("invalid navmesh tile filename")?;
            let row = row.parse::<u32>().map_err(|error| error.to_string())?;
            let col = col.parse::<u32>().map_err(|error| error.to_string())?;
            if row >= 64 || col >= 64 {
                return Err(format!("invalid navmesh tile {name}"));
            }
            tiles.push((row, col));
        }
        tiles.sort();
        if tiles.is_empty() {
            return Err(format!("{}: no navmesh tiles", self.dir.display()));
        }
        for tile in tiles {
            self.load_tile(tile)?;
        }
        Ok(())
    }

    fn build_portal_graph(&self) -> Result<PortalGraph, String> {
        let count = unsafe { (*self.mesh).getMaxTiles() as usize };
        let mut groups = vec![Vec::new(); count];
        let mut graph = PortalGraph::default();
        for (slot, group) in groups.iter_mut().enumerate() {
            let tile = unsafe { (*self.mesh).getTile(slot as i32) };
            if tile.is_null() || unsafe { (*tile).header.is_null() } {
                continue;
            }
            *group = unsafe { build_tile_clusters(tile, slot, &mut graph.clusters)? };
        }
        let mut portals = Vec::new();
        for (slot, group) in groups.iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            let tile = unsafe { (*self.mesh).getTile(slot as i32) };
            unsafe {
                append_tile_portals(
                    self.mesh,
                    tile,
                    slot,
                    group,
                    &groups,
                    &mut graph.nodes,
                    &mut portals,
                )?;
            }
        }
        connect_cross_tile_portals(self, &groups, &mut graph.nodes, &mut portals)?;
        self.connect_cluster_portals(&groups, &portals, &mut graph)?;
        Ok(graph)
    }

    fn connect_cluster_portals(
        &self,
        groups: &[Vec<u32>],
        portals: &[RawPortal],
        graph: &mut PortalGraph,
    ) -> Result<(), String> {
        let mut cluster_nodes = vec![Vec::<usize>::new(); graph.clusters.len()];
        for (index, node) in graph.nodes.iter().enumerate() {
            cluster_nodes[node.cluster as usize].push(index);
        }
        for (slot, group) in groups.iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            let tile = unsafe { (*self.mesh).getTile(slot as i32) };
            let raw =
                unsafe { std::slice::from_raw_parts((*tile).data, (*tile).dataSize as usize) };
            let mut local = NavigationMap::open(&self.dir)?;
            local.add_packet(raw)?;
            let header = unsafe { &*(*tile).header };
            let local_tile = unsafe { (*local.mesh).getTileAt(header.x, header.y, header.layer) };
            if local_tile.is_null() {
                return Err("offline local tile missing".into());
            }
            let local_base = unsafe { (*local.mesh).getPolyRefBase(local_tile) };
            let clusters: std::collections::BTreeSet<_> = group
                .iter()
                .copied()
                .filter(|&id| id != NO_CLUSTER)
                .collect();
            for cluster in clusters {
                let members = &cluster_nodes[cluster as usize];
                for &from in members {
                    for &to in members {
                        if from == to {
                            continue;
                        }
                        let from_poly = portals[from].from & ((1u64 << DT_POLY_BITS) - 1);
                        let to_poly = portals[to].from & ((1u64 << DT_POLY_BITS) - 1);
                        let start = graph.nodes[from].position;
                        let end = graph.nodes[to].position;
                        let cost = local.local_portal_cost(
                            local_base | from_poly,
                            local_base | to_poly,
                            start,
                            end,
                        )?;
                        graph.nodes[from].edges.push(PortalEdge {
                            destination: to as u32,
                            cost,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn local_portal_cost(
        &mut self,
        start_ref: dtPolyRef,
        end_ref: dtPolyRef,
        start: [f32; 3],
        end: [f32; 3],
    ) -> Result<f32, String> {
        let filter = filter();
        let mut corridor = [0; MAX_PATH];
        let mut count = 0;
        let result = unsafe {
            (*self.query).findPath(
                start_ref,
                end_ref,
                start.as_ptr(),
                end.as_ptr(),
                &filter,
                corridor.as_mut_ptr(),
                &mut count,
                MAX_PATH as i32,
            )
        };
        status(result, "offline portal path")?;
        if result & (DT_BUFFER_TOO_SMALL | DT_PARTIAL_RESULT | DT_OUT_OF_NODES) != 0
            || count == 0
            || corridor[count as usize - 1] != end_ref
        {
            return Err(format!(
                "offline portal path incomplete between {start:?} and {end:?}"
            ));
        }
        self.weighted_portal_length(&start, &end, &corridor[..count as usize])
    }

    fn weighted_portal_length(
        &self,
        start: &[f32; 3],
        end: &[f32; 3],
        corridor: &[dtPolyRef],
    ) -> Result<f32, String> {
        let mut corners = [0.0; MAX_PATH * 3];
        let mut references = [0; MAX_PATH];
        let mut count = 0;
        let result = unsafe {
            (*self.query).findStraightPath(
                start.as_ptr(),
                end.as_ptr(),
                corridor.as_ptr(),
                corridor.len() as i32,
                corners.as_mut_ptr(),
                std::ptr::null_mut(),
                references.as_mut_ptr(),
                &mut count,
                MAX_PATH as i32,
                dtStraightPathOptions_DT_STRAIGHTPATH_ALL_CROSSINGS as i32,
            )
        };
        status(result, "offline portal corners")?;
        if result & DT_BUFFER_TOO_SMALL != 0 {
            return Err("offline portal corner buffer exhausted".into());
        }
        let points = self.project_corner_surfaces(
            &corners[..count as usize * 3],
            &references[..count as usize],
            *corridor.last().ok_or("empty portal corridor")?,
        )?;
        let mut cost = 0.0;
        for (index, pair) in points.windows(2).enumerate() {
            let reference = if references[index] == 0 {
                *corridor.last().unwrap()
            } else {
                references[index]
            };
            let mut tile = std::ptr::null();
            let mut polygon = std::ptr::null();
            unsafe {
                status(
                    (*self.mesh).getTileAndPolyByRef(reference, &mut tile, &mut polygon),
                    "offline portal polygon",
                )?;
            }
            let area = unsafe { (*polygon).areaAndtype & 0x3f };
            cost += pair[0].distance(pair[1]) * if area == WATER_AREA { 2.0 } else { 1.0 };
        }
        Ok(cost.max(Vec3::from_array(*start).distance(Vec3::from_array(*end))))
    }
}

pub(super) unsafe fn build_tile_clusters(
    tile: *const dtMeshTile,
    slot: usize,
    clusters: &mut Vec<PolygonKey>,
) -> Result<Vec<u32>, String> {
    let tile = unsafe { &*tile };
    let header = unsafe { &*tile.header };
    let polygons = unsafe { std::slice::from_raw_parts(tile.polys, header.polyCount as usize) };
    let mut groups = vec![NO_CLUSTER; polygons.len()];
    for root in 0..polygons.len() {
        if groups[root] != NO_CLUSTER || !walkable(&polygons[root]) {
            continue;
        }
        let id = u32::try_from(clusters.len()).map_err(|_| "too many hierarchy clusters")?;
        clusters.push(PolygonKey {
            tile_x: header.x,
            tile_z: header.y,
            layer: header.layer,
            polygon: root as u32,
        });
        groups[root] = id;
        let mut pending = vec![root];
        while let Some(index) = pending.pop() {
            let mut link = polygons[index].firstLink;
            while link != DT_NULL_LINK {
                let current = unsafe { tile.links.add(link as usize).read_unaligned() };
                link = current.next;
                let Some((other_tile, other_poly)) = decode_ref(current.ref_) else {
                    continue;
                };
                if other_tile != slot
                    || other_poly >= polygons.len()
                    || groups[other_poly] != NO_CLUSTER
                    || !walkable(&polygons[other_poly])
                {
                    continue;
                }
                groups[other_poly] = id;
                pending.push(other_poly);
            }
        }
    }
    Ok(groups)
}

fn walkable(polygon: &dtPoly) -> bool {
    polygon.flags & WALK_FLAG != 0
        && polygon.areaAndtype >> 6 == dtPolyTypes_DT_POLYTYPE_GROUND as u8
}

fn decode_ref(reference: dtPolyRef) -> Option<(usize, usize)> {
    if reference == 0 {
        return None;
    }
    let tile = (reference >> DT_POLY_BITS) & ((1u64 << DT_TILE_BITS) - 1);
    let polygon = reference & ((1u64 << DT_POLY_BITS) - 1);
    Some((tile as usize, polygon as usize))
}

unsafe fn append_tile_portals(
    mesh: *const dtNavMesh,
    tile: *const dtMeshTile,
    slot: usize,
    group: &[u32],
    groups: &[Vec<u32>],
    nodes: &mut Vec<PortalNode>,
    portals: &mut Vec<RawPortal>,
) -> Result<(), String> {
    let tile = unsafe { &*tile };
    let header = unsafe { &*tile.header };
    let polygons = unsafe { std::slice::from_raw_parts(tile.polys, header.polyCount as usize) };
    let base = unsafe { (*mesh).getPolyRefBase(tile) };
    for (index, polygon) in polygons.iter().enumerate() {
        if group[index] == NO_CLUSTER {
            continue;
        }
        let mut link = polygon.firstLink;
        while link != DT_NULL_LINK {
            let current = unsafe { tile.links.add(link as usize).read_unaligned() };
            link = current.next;
            let Some((other_tile, other_poly)) = decode_ref(current.ref_) else {
                continue;
            };
            if other_tile == slot
                || groups
                    .get(other_tile)
                    .and_then(|g| g.get(other_poly))
                    .is_none_or(|&id| id == NO_CLUSTER)
            {
                continue;
            }
            let edge = current.edge as usize;
            if edge >= polygon.vertCount as usize {
                return Err("invalid portal edge".into());
            }
            let vertices =
                unsafe { std::slice::from_raw_parts(tile.verts, header.vertCount as usize * 3) };
            let a = polygon.verts[edge] as usize * 3;
            let b = polygon.verts[(edge + 1) % polygon.vertCount as usize] as usize * 3;
            let start = Vec3::from_slice(&vertices[a..a + 3]);
            let end = Vec3::from_slice(&vertices[b..b + 3]);
            let fraction = if current.side == 0xff {
                0.5
            } else {
                (f32::from(current.bmin) + f32::from(current.bmax)) / 510.0
            };
            let position = start.lerp(end, fraction);
            u32::try_from(nodes.len()).map_err(|_| "too many hierarchy portals")?;
            nodes.push(PortalNode {
                polygon: PolygonKey {
                    tile_x: header.x,
                    tile_z: header.y,
                    layer: header.layer,
                    polygon: index as u32,
                },
                cluster: group[index],
                position: position.to_array(),
                edges: Vec::new(),
            });
            portals.push(RawPortal {
                from: base | index as u64,
                to: current.ref_,
            });
        }
    }
    Ok(())
}

fn connect_cross_tile_portals(
    map: &NavigationMap,
    groups: &[Vec<u32>],
    nodes: &mut Vec<PortalNode>,
    portals: &mut Vec<RawPortal>,
) -> Result<(), String> {
    let mut lookup: HashMap<(dtPolyRef, dtPolyRef), Vec<usize>> = HashMap::new();
    for (index, portal) in portals.iter().enumerate() {
        lookup
            .entry((portal.from, portal.to))
            .or_default()
            .push(index);
    }
    for index in 0..portals.len() {
        let from_ref = portals[index].from;
        let to_ref = portals[index].to;
        let from = Vec3::from_array(nodes[index].position);
        let best = if let Some(reverse) = lookup.get(&(to_ref, from_ref)) {
            *reverse
                .iter()
                .min_by(|&&a, &&b| {
                    from.distance_squared(Vec3::from_array(nodes[a].position))
                        .total_cmp(&from.distance_squared(Vec3::from_array(nodes[b].position)))
                })
                .ok_or("empty reverse portal")?
        } else {
            let (tile_index, polygon_index) =
                decode_ref(to_ref).ok_or("invalid destination portal")?;
            let cluster = groups[tile_index][polygon_index];
            let mut tile = std::ptr::null();
            let mut poly = std::ptr::null();
            unsafe {
                status(
                    (*map.mesh).getTileAndPolyByRef(to_ref, &mut tile, &mut poly),
                    "destination portal polygon",
                )?;
            }
            let header = unsafe { &*(*tile).header };
            let mut position = [0.0; 3];
            unsafe {
                status(
                    (*map.query).closestPointOnPoly(
                        to_ref,
                        from.to_array().as_ptr(),
                        position.as_mut_ptr(),
                        std::ptr::null_mut(),
                    ),
                    "destination portal position",
                )?;
            }
            let id = u32::try_from(nodes.len()).map_err(|_| "too many hierarchy portals")?;
            nodes.push(PortalNode {
                polygon: PolygonKey {
                    tile_x: header.x,
                    tile_z: header.y,
                    layer: header.layer,
                    polygon: polygon_index as u32,
                },
                cluster,
                position,
                edges: Vec::new(),
            });
            portals.push(RawPortal {
                from: to_ref,
                to: 0,
            });
            id as usize
        };
        let to = Vec3::from_array(nodes[best].position);
        nodes[index].edges.push(PortalEdge {
            destination: best as u32,
            cost: from.distance(to),
        });
    }
    Ok(())
}
