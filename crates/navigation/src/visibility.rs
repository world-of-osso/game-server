//! DetourPathCorridor::optimizePathVisibility principle: tile-corner A* corridors
//! can be non-optimal because findPath prices portal midpoints, not funnel crossings.
//! Replace only with a complete native surface corridor to the same polygon/floor.
use super::*;

impl NavigationMap {
    pub(super) fn optimize_visible_corridor(
        &self,
        current: Corridor,
        filter: &dtQueryFilter,
    ) -> Result<Corridor, String> {
        let Some(polygons) = self.visible_polygons(&current, filter)? else {
            return Ok(current);
        };
        let candidate = Corridor {
            polygons,
            start: current.start,
            end: current.end,
        };
        // Visibility must not replace the existing preference for lower-cost areas.
        let (old_min, _) = self.area_cost_range(&current.polygons, filter)?;
        let (_, new_max) = self.area_cost_range(&candidate.polygons, filter)?;
        if new_max <= old_min
            && self.corridor_length(&candidate)? < self.corridor_length(&current)?
        {
            Ok(candidate)
        } else {
            Ok(current)
        }
    }

    fn visible_polygons(
        &self,
        corridor: &Corridor,
        filter: &dtQueryFilter,
    ) -> Result<Option<Vec<dtPolyRef>>, String> {
        let mut polygons = [0; MAX_PATH];
        let mut count = 0;
        let mut hit = 0.0;
        let mut normal = [0.0; 3];
        // SAFETY: validated live start ref, projected endpoints and bounded output arrays.
        let result = unsafe {
            (*self.query).raycast(
                corridor.polygons[0],
                corridor.start.as_ptr(),
                corridor.end.as_ptr(),
                filter,
                &mut hit,
                normal.as_mut_ptr(),
                polygons.as_mut_ptr(),
                &mut count,
                MAX_PATH as i32,
            )
        };
        status(result, "corridor visibility")?;
        if result & DT_BUFFER_TOO_SMALL != 0 {
            return Err("visibility corridor buffer exhausted".into());
        }
        let visited = &polygons[..count as usize];
        let same_floor = visited.last() == corridor.polygons.last();
        if hit > 1.0 && !visited.is_empty() && same_floor {
            Ok(Some(visited.to_vec()))
        } else {
            Ok(None)
        }
    }

    fn corridor_length(&self, corridor: &Corridor) -> Result<f32, String> {
        let points = self.pull_corners(&corridor.start, &corridor.end, &corridor.polygons)?;
        Ok(points
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum())
    }

    fn area_cost_range(
        &self,
        polygons: &[dtPolyRef],
        filter: &dtQueryFilter,
    ) -> Result<(f32, f32), String> {
        let mut minimum = f32::INFINITY;
        let mut maximum = 0.0_f32;
        for &reference in polygons {
            let mut tile = std::ptr::null();
            let mut poly = std::ptr::null();
            // SAFETY: query corridor references belong to the still-live mesh; no mutation.
            let area = unsafe {
                status(
                    (*self.mesh).getTileAndPolyByRef(reference, &mut tile, &mut poly),
                    "corridor area",
                )?;
                (*poly).areaAndtype & 63
            };
            let cost = filter.m_areaCost[usize::from(area)];
            minimum = minimum.min(cost);
            maximum = maximum.max(cost);
        }
        Ok((minimum, maximum))
    }
}
