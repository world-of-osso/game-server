//! Live arena and transport GO hulls: TC GameObject::EnableCollision / DynamicMapTree.
//! Cache authored M2/WMO collision, never render meshes or GeoBox substitutes.
use crate::{ground::GroundMap, line_of_sight::LineOfSight, maps::InMap};
use bevy::{math::Affine3A, prelude::*};
use shared::{
    components::{Position, Rotation},
    los::LosTile,
    protocol::{GameObjectCollision, GameObjectInfo, GameObjectVisible},
};
use std::{collections::HashMap, path::Path, sync::Arc};

pub(crate) type LiveHulls = HashMap<InMap, Vec<Arc<LosTile>>>;

#[derive(Component)]
pub(crate) struct LiveCollisionModel;

pub(crate) struct CollisionIndexPlugin;

impl Plugin for CollisionIndexPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(index_collision_model);
    }
}

// GameObjectInfo and map membership are installed/replaced as components; movement,
// visibility and collision enablement remain live reads of the indexed objects.
pub(crate) fn index_collision_model(
    event: On<Insert, (GameObjectInfo, InMap)>,
    objects: Query<(&GameObjectInfo, &InMap)>,
    catalog: Option<Res<crate::pvp::arena::Catalog>>,
    mut commands: Commands,
) {
    let Ok((info, map)) = objects.get(event.entity) else {
        return;
    };
    let arena = catalog
        .as_ref()
        .is_some_and(|catalog| catalog.contains_map(map.map_id));
    let relevant = info.go_type == 15 || (arena && matches!(info.go_type, 0 | 5 | 6 | 35));
    if relevant {
        commands.entity(event.entity).insert(LiveCollisionModel);
    } else {
        commands.entity(event.entity).remove::<LiveCollisionModel>();
    }
}

#[cfg(test)]
#[path = "collision_tests.rs"]
mod tests;
type Objects<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InMap,
        &'static Position,
        &'static Rotation,
        &'static GameObjectInfo,
        &'static GameObjectCollision,
        Option<&'static GameObjectVisible>,
    ),
    With<LiveCollisionModel>,
>;

#[derive(Clone, PartialEq)]
struct Placement {
    display: u32,
    go_type: u8,
    position: Vec3,
    yaw: f32,
    scale: f32,
}
#[derive(Default)]
pub(crate) struct HullCache {
    displays: Option<HashMap<u32, u32>>,
    models: HashMap<u32, Result<Vec<[Vec3; 3]>, String>>,
    placed: HashMap<Entity, (Placement, Arc<LosTile>)>,
}

#[cfg(test)]
#[test]
fn db2_follow_install_collision_uses_the_active_display_csv() {
    let directory = tempfile::tempdir().unwrap();
    let active = directory.path().join("db2/active");
    std::fs::create_dir_all(&active).unwrap();
    std::fs::write(
        active.join("GameObjectDisplayInfo.csv"),
        "ID,FileDataID\n47198,196519\n",
    )
    .unwrap();
    let displays = read_displays(directory.path()).unwrap();
    assert_eq!(displays[&47198], 196519);
}

fn read_displays(data: &Path) -> Result<HashMap<u32, u32>, String> {
    let file = data.join("db2/active/GameObjectDisplayInfo.csv");
    let text = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut lines = text.lines();
    let headers: Vec<_> = lines
        .next()
        .ok_or("empty GameObjectDisplayInfo")?
        .split(',')
        .collect();
    let id = headers
        .iter()
        .position(|h| *h == "ID")
        .ok_or("GameObjectDisplayInfo missing ID")?;
    let fdid = headers
        .iter()
        .position(|h| *h == "FileDataID")
        .ok_or("GameObjectDisplayInfo missing FileDataID")?;
    lines
        .filter(|line| !line.is_empty())
        .map(|line| {
            let row: Vec<_> = line.split(',').collect();
            let number = |index: usize| {
                row.get(index)
                    .ok_or_else(|| format!("incomplete display row: {line}"))?
                    .parse::<u32>()
                    .map_err(|e| e.to_string())
            };
            Ok((number(id)?, number(fdid)?))
        })
        .collect()
}

fn place_hull(triangles: &[[Vec3; 3]], placement: &Placement) -> LosTile {
    // Native GameObjectModel has -pi/2 yaw inside the replicated GO transform.
    let transform = Affine3A::from_scale_rotation_translation(
        Vec3::splat(placement.scale),
        Quat::from_rotation_y(placement.yaw - std::f32::consts::FRAC_PI_2),
        placement.position,
    );
    let vertices = triangles
        .iter()
        .flatten()
        .map(|point| transform.transform_point3(*point))
        .collect();
    let indices = (0..triangles.len() as u32)
        .map(|i| [3 * i, 3 * i + 1, 3 * i + 2])
        .collect();
    LosTile::build(0, 0, vertices, indices)
}

fn read_model_triangles(data: &Path, fdid: u32, go_type: u8) -> Result<Vec<[Vec3; 3]>, String> {
    if go_type != 15 {
        let path = data.join(format!("models/{fdid}.m2"));
        let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        return los::bake::m2::M2File::parse(&bytes)?.collision_triangles();
    }
    let path = data.join(format!("models/{fdid}.wmo"));
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let root = los::bake::wmo::WmoRoot::parse(&bytes)?;
    let mut triangles = Vec::new();
    for group in root.group_fdids {
        let path = data.join(format!("models/{group}.wmo"));
        let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let collision = shared::ground::WmoGroupCollision::parse(&bytes)?;
        triangles.extend(collision.collidable_triangles());
    }
    if triangles.is_empty() {
        return Err(format!("transport WMO {fdid} has no authored collision"));
    }
    Ok(triangles)
}

impl HullCache {
    fn load_triangles(
        &mut self,
        data: &Path,
        display: u32,
        go_type: u8,
    ) -> Result<&[[Vec3; 3]], String> {
        if self.displays.is_none() {
            self.displays = Some(read_displays(data)?);
        }
        let fdid = *self
            .displays
            .as_ref()
            .unwrap()
            .get(&display)
            .ok_or_else(|| format!("display {display} has no FileDataID"))?;
        self.models
            .entry(fdid)
            .or_insert_with(|| read_model_triangles(data, fdid, go_type))
            .as_ref()
            .map(Vec::as_slice)
            .map_err(Clone::clone)
    }

    fn load_placed_hull(
        &mut self,
        data: &Path,
        entity: Entity,
        placement: Placement,
    ) -> Result<Arc<LosTile>, String> {
        if let Some((old, hull)) = self.placed.get(&entity)
            && *old == placement
        {
            return Ok(Arc::clone(hull));
        }
        let triangles = self.load_triangles(data, placement.display, placement.go_type)?;
        let hull = Arc::new(place_hull(triangles, &placement));
        self.placed.insert(entity, (placement, Arc::clone(&hull)));
        Ok(hull)
    }
}

/// Runs before movement/gravity. Each instance receives only its own enabled hulls.
pub(crate) fn sync_live_objects(
    ground: Option<ResMut<GroundMap>>,
    los: Option<ResMut<LineOfSight>>,
    objects: Objects,
    mut cache: Local<HullCache>,
) {
    let Some(mut ground) = ground else {
        return;
    };
    let mut active = LiveHulls::new();
    cache.placed.retain(|entity, _| objects.contains(*entity));
    for (entity, map, position, rotation, info, collision, visible) in &objects {
        let enabled = collision.0 && visible.is_none_or(|v| v.0);
        if !enabled {
            continue;
        }
        let placement = Placement {
            display: info.display_id,
            go_type: info.go_type,
            position: Vec3::new(position.x, position.y, position.z),
            yaw: rotation.y,
            scale: info.scale,
        };
        match cache.load_placed_hull(ground.data_dir(), entity, placement) {
            Ok(hull) => active.entry(*map).or_default().push(hull),
            Err(error) => panic!(
                "Live GO {} (entity {entity}, map {map:?}) collision unavailable: {error}",
                info.entry
            ),
        }
    }
    ground.set_live_hulls(active.clone());
    if let Some(mut los) = los {
        los.set_live_hulls(active);
    }
}
