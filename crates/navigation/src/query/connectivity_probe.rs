//! Independent polygon-level reachability oracle for real-asset qualification.
use super::*;
use std::collections::{HashSet, VecDeque};

impl NavigationMap {
    /// Traverse actual directed Detour links, without consulting the hierarchy.
    pub fn diagnostic_reachable(
        &mut self,
        start: Vec3,
        targets: &[Vec3],
    ) -> Result<Vec<bool>, String> {
        self.load_graph_tiles()?;
        let filter = filter();
        let (start_ref, _) = self.nearest(start, &filter)?;
        let target_refs = targets
            .iter()
            .map(|&target| {
                self.nearest(target, &filter)
                    .map(|(reference, _)| reference)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut visited = HashSet::from([start_ref]);
        let mut queue = VecDeque::from([start_ref]);
        while let Some(reference) = queue.pop_front() {
            let mut tile = std::ptr::null();
            let mut polygon = std::ptr::null();
            unsafe {
                status(
                    (*self.mesh).getTileAndPolyByRef(reference, &mut tile, &mut polygon),
                    "connectivity polygon",
                )?;
            }
            let mut link = unsafe { (*polygon).firstLink };
            while link != DT_NULL_LINK {
                let item = unsafe { (*tile).links.add(link as usize).read_unaligned() };
                link = item.next;
                if item.ref_ == 0 || visited.contains(&item.ref_) {
                    continue;
                }
                let mut next_tile = std::ptr::null();
                let mut next_polygon = std::ptr::null();
                unsafe {
                    status(
                        (*self.mesh).getTileAndPolyByRef(
                            item.ref_,
                            &mut next_tile,
                            &mut next_polygon,
                        ),
                        "connectivity neighbor",
                    )?;
                }
                let polygon = unsafe { &*next_polygon };
                if polygon.flags & WALK_FLAG == 0
                    || polygon.areaAndtype >> 6 != dtPolyTypes_DT_POLYTYPE_GROUND as u8
                {
                    continue;
                }
                visited.insert(item.ref_);
                queue.push_back(item.ref_);
            }
        }
        Ok(target_refs
            .iter()
            .map(|reference| visited.contains(reference))
            .collect())
    }

    pub fn diagnostic_export_component(&mut self, start: Vec3, path: &str) -> Result<(), String> {
        use std::io::Write;
        self.load_graph_tiles()?;
        let (reference, projected) = self.nearest(start, &filter())?;
        let mut queue = VecDeque::from([reference]);
        let mut visited = HashSet::from([reference]);
        let mut out = std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| e.to_string())?);
        eprintln!("start={start:?} projected={projected:?}");
        while let Some(reference) = queue.pop_front() {
            let mut tile = std::ptr::null();
            let mut poly = std::ptr::null();
            unsafe {
                status((*self.mesh).getTileAndPolyByRef(reference, &mut tile, &mut poly), "component polygon")?;
                let p = &*poly;
                write!(out, "{} {}", reference, p.areaAndtype & 63).map_err(|e|e.to_string())?;
                for index in p.verts.iter().take(p.vertCount as usize) {
                    let v = (*tile).verts.add(*index as usize * 3);
                    write!(out, " {:.8} {:.8} {:.8}", *v, *v.add(1), *v.add(2)).map_err(|e|e.to_string())?;
                }
                writeln!(out).map_err(|e|e.to_string())?;
                let mut link = p.firstLink;
                while link != DT_NULL_LINK {
                    let item = (*tile).links.add(link as usize).read_unaligned();
                    link = item.next;
                    if item.ref_ == 0 || visited.contains(&item.ref_) { continue; }
                    let mut nt = std::ptr::null(); let mut np = std::ptr::null();
                    status((*self.mesh).getTileAndPolyByRef(item.ref_, &mut nt, &mut np), "component neighbor")?;
                    if (*np).flags & WALK_FLAG == 0 || (*np).areaAndtype >> 6 != dtPolyTypes_DT_POLYTYPE_GROUND as u8 { continue; }
                    visited.insert(item.ref_); queue.push_back(item.ref_);
                }
            }
        }
        eprintln!("exported {} polygons to {}", visited.len(), path);
        Ok(())
    }
}
