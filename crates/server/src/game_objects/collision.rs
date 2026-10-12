//! Live arena GO hulls: TC GameObject::EnableCollision / DynamicMapTree.
//! Cache authored M2 collision, never render meshes or GeoBox substitutes.
use crate::{ground::GroundMap, line_of_sight::LineOfSight, maps::InMap};
use bevy::{math::Affine3A, prelude::*};
use shared::{
    components::{Position, Rotation},
    los::LosTile,
    protocol::{GameObjectCollision, GameObjectInfo, GameObjectVisible},
};
use std::{collections::HashMap, path::Path, sync::Arc};

pub(crate) type LiveHulls = HashMap<InMap, Vec<Arc<LosTile>>>;
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
>;

#[derive(Clone, PartialEq)]
struct Placement {
    display: u32,
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

fn read_displays(data: &Path) -> Result<HashMap<u32, u32>, String> {
    let file = data.join("db2/12.1.0.69933/GameObjectDisplayInfo.csv");
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

impl HullCache {
    fn load_triangles(&mut self, data: &Path, display: u32) -> Result<&[[Vec3; 3]], String> {
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
            .or_insert_with(|| {
                let path = data.join(format!("models/{fdid}.m2"));
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                los::bake::m2::M2File::parse(&bytes)?.collision_triangles()
            })
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
        let triangles = self.load_triangles(data, placement.display)?;
        let hull = Arc::new(place_hull(triangles, &placement));
        self.placed.insert(entity, (placement, Arc::clone(&hull)));
        Ok(hull)
    }
}

/// Runs before movement/gravity. Each instance receives only its own enabled hulls.
pub(crate) fn sync_live_objects(
    ground: Option<ResMut<GroundMap>>,
    los: Option<ResMut<LineOfSight>>,
    catalog: Option<Res<crate::pvp::arena::Catalog>>,
    objects: Objects,
    mut cache: Local<HullCache>,
) {
    let Some(mut ground) = ground else {
        return;
    };
    let mut active = LiveHulls::new();
    cache.placed.retain(|entity, _| objects.contains(*entity));
    for (entity, map, position, rotation, info, collision, visible) in &objects {
        let arena = catalog
            .as_ref()
            .is_some_and(|catalog| catalog.contains_map(map.map_id));
        let has_model_collision = matches!(info.go_type, 0 | 5 | 6 | 35);
        let enabled = collision.0 && visible.is_none_or(|v| v.0);
        if !(arena && has_model_collision && enabled) {
            continue;
        }
        let placement = Placement {
            display: info.display_id,
            position: Vec3::new(position.x, position.y, position.z),
            yaw: rotation.y,
            scale: info.scale,
        };
        match cache.load_placed_hull(ground.data_dir(), entity, placement) {
            Ok(hull) => active.entry(*map).or_default().push(hull),
            Err(error) => panic!(
                "Arena GO {} (entity {entity}, map {map:?}) collision unavailable: {error}",
                info.entry
            ),
        }
    }
    ground.set_live_hulls(active.clone());
    if let Some(mut los) = los {
        los.set_live_hulls(active);
    }
}
