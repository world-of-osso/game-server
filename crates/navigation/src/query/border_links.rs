//! Reject impossible cross-tile links after Detour connects a tile.
use super::*;

impl NavigationMap {
    /// Detour accepts border gaps up to twice `walkableClimb`. A follower can step only
    /// `AGENT_CLIMB`, so remove both directions of a link whose midpoint exceeds it.
    pub(super) unsafe fn prune_border_links(&mut self, added_ref: dtTileRef) -> Result<(), String> {
        // SAFETY: the caller owns the initialized mesh and query exclusively. Detour owns
        // these tiles until NavigationMap is dropped; their links are mutable mesh state.
        unsafe {
            let added = (*self.mesh).getTileByRef(added_ref);
            if added.is_null() {
                return Err("new navigation tile is missing".into());
            }
            let header = &*(*added).header;
            let mut tiles = HashSet::new();
            let max_tiles = (*self.mesh).getMaxTiles();
            let mut neighbors = vec![std::ptr::null(); max_tiles as usize];
            for x in header.x - 1..=header.x + 1 {
                for y in header.y - 1..=header.y + 1 {
                    let count = (*self.mesh).getTilesAt(x, y, neighbors.as_mut_ptr(), max_tiles);
                    tiles.extend(
                        neighbors[..count as usize]
                            .iter()
                            .map(|tile| *tile as usize),
                    );
                }
            }
            let mut rejected = HashSet::new();
            for &address in &tiles {
                let tile = address as *const dtMeshTile;
                let base = (*self.mesh).getPolyRefBase(tile);
                let header = &*(*tile).header;
                for index in 0..header.polyCount as usize {
                    let polygon = &*(*tile).polys.add(index);
                    if polygon.areaAndtype >> 6 != dtPolyTypes_DT_POLYTYPE_GROUND as u8 {
                        continue;
                    }
                    let source = base | index as dtPolyRef;
                    let mut link = polygon.firstLink;
                    while link != DT_NULL_LINK {
                        let entry = &*(*tile).links.add(link as usize);
                        let mut target = std::ptr::null();
                        let mut target_poly = std::ptr::null();
                        status(
                            (*self.mesh).getTileAndPolyByRef(
                                entry.ref_,
                                &mut target,
                                &mut target_poly,
                            ),
                            "border target",
                        )?;
                        if target != tile
                            && (tile == added || target == added)
                            && (*target_poly).areaAndtype >> 6
                                == dtPolyTypes_DT_POLYTYPE_GROUND as u8
                            && self.border_height_gap(tile, polygon, source, entry)? > AGENT_CLIMB
                        {
                            rejected.insert(link_pair(source, entry.ref_));
                        }
                        link = entry.next;
                    }
                }
            }
            for address in tiles {
                let tile = address as *mut dtMeshTile;
                let base = (*self.mesh).getPolyRefBase(tile);
                let count = (*(*tile).header).polyCount as usize;
                for index in 0..count {
                    let polygon = &mut *(*tile).polys.add(index);
                    let source = base | index as dtPolyRef;
                    let mut cursor = &mut polygon.firstLink as *mut u32;
                    while *cursor != DT_NULL_LINK {
                        let link = *cursor;
                        let entry = &mut *(*tile).links.add(link as usize);
                        if rejected.contains(&link_pair(source, entry.ref_)) {
                            *cursor = entry.next;
                            entry.next = (*tile).linksFreeList;
                            (*tile).linksFreeList = link;
                        } else {
                            cursor = &mut entry.next;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    unsafe fn border_height_gap(
        &self,
        tile: *const dtMeshTile,
        polygon: &dtPoly,
        source: dtPolyRef,
        link: &dtLink,
    ) -> Result<f32, String> {
        // SAFETY: both polygon refs and the edge come from a live Detour tile link.
        unsafe {
            let edge = link.edge as usize;
            if edge >= polygon.vertCount as usize {
                return Err("invalid navigation border edge".into());
            }
            let vertex = |index: u16| {
                Vec3::from_slice(std::slice::from_raw_parts(
                    (*tile).verts.add(index as usize * 3),
                    3,
                ))
            };
            let a = vertex(polygon.verts[edge]);
            let b = vertex(polygon.verts[(edge + 1) % polygon.vertCount as usize]);
            let fraction = if link.side == 0xff {
                0.5
            } else {
                (f32::from(link.bmin) + f32::from(link.bmax)) / 510.0
            };
            let point = a.lerp(b, fraction).to_array();
            let mut from = [0.0; 3];
            let mut to = [0.0; 3];
            status(
                (*self.query).closestPointOnPoly(
                    source,
                    point.as_ptr(),
                    from.as_mut_ptr(),
                    std::ptr::null_mut(),
                ),
                "source border height",
            )?;
            status(
                (*self.query).closestPointOnPoly(
                    link.ref_,
                    point.as_ptr(),
                    to.as_mut_ptr(),
                    std::ptr::null_mut(),
                ),
                "target border height",
            )?;
            Ok((from[1] - to[1]).abs())
        }
    }
}

fn link_pair(a: dtPolyRef, b: dtPolyRef) -> (dtPolyRef, dtPolyRef) {
    (a.min(b), a.max(b))
}
