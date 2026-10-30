//! Complete a walking route through the same segment continuation as the mover.
use std::collections::HashSet;

use glam::Vec3;

use crate::{NavigationMap, NavigationPlan};

const MAX_COMPLETE_ROUTE_SEGMENTS: usize = 256;

#[derive(Clone, Debug)]
pub struct CompleteRoute {
    pub distance: f32,
    pub complete: bool,
    pub reason: Option<String>,
}

fn incomplete(distance: f32, reason: impl Into<String>) -> CompleteRoute {
    CompleteRoute {
        distance,
        complete: false,
        reason: Some(reason.into()),
    }
}

pub(crate) fn follow_complete_route(
    map: &mut NavigationMap,
    from: Vec3,
    target: Vec3,
) -> CompleteRoute {
    let mut current = from;
    let mut distance = 0.0;
    let mut endpoints = HashSet::new();
    let mut plan = NavigationPlan::default();
    for _ in 0..MAX_COMPLETE_ROUTE_SEGMENTS {
        // Each path query loads the current segment's endpoint neighborhoods.
        let path = match map.path_with_plan(current, target, &mut plan) {
            Ok(path) => path,
            Err(error) => return incomplete(distance, error),
        };
        let Some(&endpoint) = path.points.last() else {
            return incomplete(distance, "empty navigation segment");
        };
        let mut previous = current;
        for point in path.points {
            distance += previous.distance(point);
            previous = point;
        }
        if path.complete {
            return CompleteRoute {
                distance,
                complete: true,
                reason: None,
            };
        }
        if endpoint == current || !endpoints.insert(endpoint.to_array().map(f32::to_bits)) {
            return incomplete(distance, "partial corridor made no progress or repeated");
        }
        current = endpoint;
    }
    incomplete(distance, "complete route segment limit reached")
}
