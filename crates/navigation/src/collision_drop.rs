//! NAV_ONE_WAY_DROP_FROM_COLLISION: the single lead-approved2651 connection.
//! Raw upper edge(534.269,86.534424,110); native source...538/target...401.
//! Departure clears the lip with the unchanged body radius; server MoveFall owns descent.
use super::{NavigationMap, NavigationPath};
use glam::Vec3;

const MAP_DIRECTORY: &str = "2651";
const APPROACH: Vec3 = Vec3::new(535.1, 86.534424, 110.0);
const DEPARTURE: Vec3 = Vec3::new(533.88, 86.534424, 110.0);
const LANDING: Vec3 = Vec3::new(533.88, 56.281814, 110.0);

/// An explicit off-mesh transition after a native walking prefix, not walking corners.
#[derive(Clone, Debug, PartialEq)]
pub struct CollisionDrop {
    pub after_corner: usize,
    pub departure: Vec3,
    pub landing: Vec3,
}

impl NavigationMap {
    pub(super) fn has_collision_drop(&self) -> bool {
        self.dir.file_name().and_then(|name| name.to_str()) == Some(MAP_DIRECTORY)
    }

    /// Search the one directed graph edge using native walking queries on both sides.
    pub(super) fn path_via_collision_drop(
        &mut self,
        start: Vec3,
        end: Vec3,
    ) -> Result<NavigationPath, String> {
        let mut prefix = self.walking_path(start, APPROACH)?;
        if !prefix.complete {
            return Ok(prefix);
        }
        let suffix = self.walking_path(LANDING, end)?;
        validate_endpoint(prefix.points.last(), APPROACH)?;
        validate_endpoint(suffix.points.first(), LANDING)?;
        let after_corner = prefix.points.len() - 1;
        prefix.points.extend([DEPARTURE, LANDING]);
        prefix.points.extend(suffix.points.into_iter().skip(1));
        prefix.complete = suffix.complete;
        prefix.collision_drop = Some(CollisionDrop {
            after_corner,
            departure: DEPARTURE,
            landing: LANDING,
        });
        Ok(prefix)
    }
}

fn validate_endpoint(point: Option<&Vec3>, expected: Vec3) -> Result<(), String> {
    if point.is_some_and(|point| point.distance(expected) <= crate::CELL_HEIGHT) {
        Ok(())
    } else {
        Err(
            "NAV_ONE_WAY_DROP_FROM_COLLISION: native endpoint no longer matches qualified support"
                .into(),
        )
    }
}
