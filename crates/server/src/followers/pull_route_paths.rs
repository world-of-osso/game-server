//! Serialized admission geometry on the ordinary navigation worker; pending never moves.
use crate::maps::InMap;
use crate::navigation::{Goal, Navigation};
use ::navigation::worker::{Output, Query};
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Target(u32),
    Point,
}

#[derive(Clone)]
struct CapturedPath {
    key: Key,
    from: Vec3,
    to: Vec3,
}

struct CompletedPath {
    captured: CapturedPath,
    result: Result<Vec<Vec3>, String>,
}

#[derive(Component)]
struct AdmissionPaths {
    map: InMap,
    pending: Option<CapturedPath>,
    completed: BTreeMap<Key, CompletedPath>,
}

impl AdmissionPaths {
    fn poll(&mut self, nav: &mut Navigation, tank: Entity) -> bool {
        let Some(captured) = self.pending.clone() else {
            return true;
        };
        if nav.query_owner(tank) != Some(Goal::PullAdmission) {
            self.pending = None;
            return true;
        }
        let Some(result) = nav.query_deferred(
            tank,
            self.map.map_id,
            Goal::PullAdmission,
            Query::Path {
                from: captured.from,
                to: captured.to,
            },
        ) else {
            return false;
        };
        let result = result.and_then(|output| match output {
            Some(Output::Path(points)) => Ok(points),
            None => Err("Pull route has no authored navigation bake".into()),
            Some(_) => Err("Pull route worker returned a non-path result".into()),
        });
        self.completed
            .insert(captured.key, CompletedPath { captured, result });
        self.pending = None;
        nav.cancel_query(tank, Goal::PullAdmission);
        true
    }

    fn read(
        &mut self,
        nav: &mut Navigation,
        tank: Entity,
        captured: CapturedPath,
    ) -> Result<Option<Vec<Vec3>>, String> {
        if !self.poll(nav, tank) {
            return Ok(None);
        }
        if let Some(ready) = self.completed.get(&captured.key)
            && ready.captured.from == captured.from
            && ready.captured.to == captured.to
        {
            return ready.result.clone().map(Some);
        }
        // Never evict an unfinished chase, escape, fear or other native continuation.
        if nav.route_pending(tank) {
            return Ok(None);
        }
        let _ = nav.query_deferred(
            tank,
            self.map.map_id,
            Goal::PullAdmission,
            Query::Path {
                from: captured.from,
                to: captured.to,
            },
        );
        self.pending = Some(captured);
        Ok(None)
    }
}

pub(super) fn read(
    world: &mut World,
    tank: Entity,
    map: InMap,
    key: Key,
    from: Vec3,
    to: Vec3,
) -> Result<Option<Vec<Vec3>>, String> {
    let mut paths = world
        .entity_mut(tank)
        .take::<AdmissionPaths>()
        .filter(|paths| paths.map == map)
        .unwrap_or_else(|| AdmissionPaths {
            map,
            pending: None,
            completed: BTreeMap::new(),
        });
    let result = world
        .get_resource_mut::<Navigation>()
        .ok_or_else(|| "Pull route requires authored navigation".to_owned())
        .and_then(|mut nav| paths.read(&mut nav, tank, CapturedPath { key, from, to }));
    world.entity_mut(tank).insert(paths);
    result
}
