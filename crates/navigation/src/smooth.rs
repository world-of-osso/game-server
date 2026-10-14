//! TC a352b1fa PathGenerator::{FindSmoothPath,GetSteerTarget,FixupCorridor}.
//! Dedicated C++ ground waypoint controls; generic string-pulled routes are unchanged.
use super::*;

const SOURCE_PATH_LIMIT: usize = 74;
const STEP_SIZE: f32 = 4.0;
const SLOP: f32 = 0.3;
const STEER_POINTS: usize = 3;
const VISITED_POLYGONS: usize = 16;
const HEIGHT_OFFSET: f32 = 0.5;
const STEER_HEIGHT_SLOP: f32 = 1000.0;
const END_HEIGHT_SLOP: f32 = 1.0;

struct Steering {
    point: Vec3,
    end: bool,
}

impl NavigationMap {
    pub(super) fn find_smooth_path(
        &self,
        start: Vec3,
        end: Vec3,
        mut polygons: Vec<dtPolyRef>,
    ) -> Result<Vec<Vec3>, String> {
        if polygons.len() > SOURCE_PATH_LIMIT {
            return Err("waypoint corridor exceeds source 74-polygon limit".into());
        }
        let at = self.boundary_point(polygons[0], start, polygons.len() > 1)?;
        let target = self.boundary_point(*polygons.last().unwrap(), end, polygons.len() > 1)?;
        let points = self.collect_surface_points(at, target, &mut polygons)?;
        validate_point_count(points)
    }

    fn collect_surface_points(
        &self,
        mut at: Vec3,
        target: Vec3,
        polygons: &mut Vec<dtPolyRef>,
    ) -> Result<Vec<Vec3>, String> {
        let mut points = vec![at];
        while points.len() < SOURCE_PATH_LIMIT {
            let Some(steering) = self.query_steering(at, target, polygons)? else {
                // BuildPointPath's same-polygon, one-point special case.
                if points.len() == 1 && polygons.len() == 1 {
                    points.push(target);
                }
                // FindSmoothPath breaks here, then BuildPointPath validates point count.
                // A final surface step may reach target XZ before its height test passes;
                // NormalizePath subsequently corrects those feet to collision support.
                break;
            };
            at = self.step_surface(at, &steering, polygons)?;
            if steering.end && in_range(at, steering.point, SLOP, END_HEIGHT_SLOP) {
                points.push(target);
                break;
            }
            points.push(at);
        }
        Ok(points)
    }

    fn boundary_point(
        &self,
        polygon: dtPolyRef,
        point: Vec3,
        project: bool,
    ) -> Result<Vec3, String> {
        if !project {
            return Ok(point);
        }
        let mut boundary = [0.0; 3];
        // SAFETY: query initialized; corridor refs validated; three-float buffers.
        unsafe {
            status(
                (*self.query).closestPointOnPolyBoundary(
                    polygon,
                    point.to_array().as_ptr(),
                    boundary.as_mut_ptr(),
                ),
                "waypoint boundary projection",
            )?;
        }
        Ok(Vec3::from_array(boundary))
    }

    fn query_steering(
        &self,
        at: Vec3,
        target: Vec3,
        polygons: &[dtPolyRef],
    ) -> Result<Option<Steering>, String> {
        let corners = self.query_steering_corners(at, target, polygons)?;
        for (mut point, flags) in corners {
            let off_mesh =
                flags & dtStraightPathFlags_DT_STRAIGHTPATH_OFFMESH_CONNECTION as u8 != 0;
            if off_mesh {
                return Err("off-mesh C++ waypoint movement is not ported".into());
            }
            if !in_range(point, at, SLOP, STEER_HEIGHT_SLOP) {
                point.y = at.y;
                let end = flags & dtStraightPathFlags_DT_STRAIGHTPATH_END as u8 != 0;
                return Ok(Some(Steering { point, end }));
            }
        }
        Ok(None)
    }

    fn query_steering_corners(
        &self,
        at: Vec3,
        target: Vec3,
        polygons: &[dtPolyRef],
    ) -> Result<Vec<(Vec3, u8)>, String> {
        let mut points = [0.0; STEER_POINTS * 3];
        let mut flags = [0; STEER_POINTS];
        let mut count = 0;
        // SAFETY: initialized query, valid corridor, bounded output buffers.
        unsafe {
            status(
                (*self.query).findStraightPath(
                    at.to_array().as_ptr(),
                    target.to_array().as_ptr(),
                    polygons.as_ptr(),
                    polygons.len() as i32,
                    points.as_mut_ptr(),
                    flags.as_mut_ptr(),
                    std::ptr::null_mut(),
                    &mut count,
                    STEER_POINTS as i32,
                    0,
                ),
                "waypoint steering",
            )?;
        }
        Ok(points[..count as usize * 3]
            .chunks_exact(3)
            .zip(flags)
            .map(|(point, flags)| (Vec3::from_slice(point), flags))
            .collect())
    }

    fn step_surface(
        &self,
        at: Vec3,
        steering: &Steering,
        polygons: &mut Vec<dtPolyRef>,
    ) -> Result<Vec3, String> {
        let delta = steering.point - at;
        let distance = delta.length();
        let scale = if steering.end && distance < STEP_SIZE {
            1.0
        } else {
            STEP_SIZE / distance
        };
        let target = at + delta * scale;
        let (point, visited) = self.query_surface_step(polygons[0], at, target)?;
        fixup_corridor(polygons, &visited)?;
        self.project_surface_height(polygons[0], point)
    }

    fn query_surface_step(
        &self,
        polygon: dtPolyRef,
        at: Vec3,
        target: Vec3,
    ) -> Result<(Vec3, Vec<dtPolyRef>), String> {
        let mut result = [0.0; 3];
        let mut visited = [0; VISITED_POLYGONS];
        let mut count = 0;
        // SAFETY: current ref valid, initialized query; fixed-size output buffers.
        let status_code = unsafe {
            (*self.query).moveAlongSurface(
                polygon,
                at.to_array().as_ptr(),
                target.to_array().as_ptr(),
                &filter(),
                result.as_mut_ptr(),
                visited.as_mut_ptr(),
                &mut count,
                VISITED_POLYGONS as i32,
            )
        };
        status(status_code, "waypoint surface movement")?;
        if status_code & DT_BUFFER_TOO_SMALL != 0 || count == 0 {
            return Err("incomplete waypoint surface movement".into());
        }
        Ok((Vec3::from_array(result), visited[..count as usize].to_vec()))
    }

    fn project_surface_height(&self, polygon: dtPolyRef, point: Vec3) -> Result<Vec3, String> {
        let mut height = 0.0;
        // SAFETY: fixup retains validated mesh refs; point has three floats.
        unsafe {
            status(
                (*self.query).getPolyHeight(polygon, point.to_array().as_ptr(), &mut height),
                "waypoint surface height",
            )?;
        }
        Ok(Vec3::new(point.x, height + HEIGHT_OFFSET, point.z))
    }
}

fn validate_point_count(points: Vec<Vec3>) -> Result<Vec<Vec3>, String> {
    if points.len() >= SOURCE_PATH_LIMIT {
        return Err("waypoint surface path exhausted source 74-point limit".into());
    }
    if points.len() < 2 {
        return Err("waypoint surface path has fewer than two controls".into());
    }
    Ok(points)
}

fn in_range(a: Vec3, b: Vec3, radius: f32, height: f32) -> bool {
    let delta = b - a;
    delta.x * delta.x + delta.z * delta.z < radius * radius && delta.y.abs() < height
}

/// Reattach the surviving corridor to the furthest polygon visited by the surface step.
fn fixup_corridor(polygons: &mut Vec<dtPolyRef>, visited: &[dtPolyRef]) -> Result<(), String> {
    let common = polygons
        .iter()
        .enumerate()
        .rev()
        .find_map(|(path_index, polygon)| {
            visited
                .iter()
                .rposition(|other| other == polygon)
                .map(|visited_index| (path_index, visited_index))
        });
    let Some((path_index, visited_index)) = common else {
        return Ok(());
    };
    let prefix: Vec<_> = visited[visited_index..].iter().rev().copied().collect();
    let remaining = &polygons[path_index + 1..];
    if prefix.len() + remaining.len() > SOURCE_PATH_LIMIT {
        return Err("waypoint surface corridor exceeds source limit".into());
    }
    *polygons = prefix
        .into_iter()
        .chain(remaining.iter().copied())
        .collect();
    Ok(())
}
