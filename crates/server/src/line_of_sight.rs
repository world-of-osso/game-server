//! Line of sight (docs/specs/line-of-sight.md) over the baked LOS tiles (`shared::los`):
//! creatures and player casts search from authoritative eye points to the target's
//! eyes, chest and feet (LOS holds when any ray is clear).
//!
//! The bake output (`los-bake --out <ground dir>/los`) loads from the ground directory:
//! the eye table at startup, each map's tiles on first use. A map without baked tiles
//! has nothing that blocks. The bake covers every spawned and player display and the
//! creature template displays it can; a unit whose display has no eye row (a skipped
//! display a script summons) is logged once per display, naming the first unit that
//! showed it, and sees nothing and is seen by nothing.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::math::Vec3;
use bevy::prelude::*;
use los::eye::EyeTable;
use los::{Body, LosMap};
use shared::components::{
    ModelDisplay, Npc, Player, PlayerStandState, Position, Rotation, StandState, UnitPose,
};

use crate::maps::InMap;

/// The ground directory's subdirectory the bake writes to.
const LOS_DIR: &str = "los";

#[derive(Resource)]
pub struct LineOfSight {
    dir: PathBuf,
    eyes: EyeTable,
    /// Baked tiles per map id, loaded on first use.
    maps: Mutex<HashMap<u32, LosMap>>,
    /// Displays without an eye row, logged once each.
    missing: Mutex<HashSet<u32>>,
    /// Segments tested against the bake so far.
    rays: AtomicU64,
    live_hulls: crate::game_objects::collision::LiveHulls,
}

/// What places a unit's eyes: its components (the entity and `Npc` name it in logs).
pub(crate) type BodyParts<'a> = (
    Entity,
    Option<&'a Npc>,
    &'a Position,
    Option<&'a Rotation>,
    Option<&'a ModelDisplay>,
    Option<&'a UnitPose>,
    Option<&'a Player>,
    Option<&'a PlayerStandState>,
);

impl LineOfSight {
    /// The bake output in `dir`: `eye_heights.tsv`, `player_displays.tsv`, `<map>/`.
    pub fn load(dir: &Path) -> Result<Self, String> {
        Ok(Self {
            dir: dir.to_path_buf(),
            eyes: EyeTable::load(dir)?,
            maps: Mutex::new(HashMap::new()),
            missing: Mutex::new(HashSet::new()),
            rays: AtomicU64::new(0),
            live_hulls: HashMap::new(),
        })
    }

    /// The unit's feet and eyes: the eye point of its display (a player's race and sex
    /// display) in its stand state, turned to its facing. `None`, logged once, without an
    /// eye row.
    pub fn body(
        &self,
        (entity, npc, position, rotation, display, pose, player, player_stand): BodyParts,
    ) -> Option<Body> {
        let (display, stand_state) = match (player, display) {
            (Some(player), _) => (
                self.eyes
                    .player_display(player.race, player.appearance.sex)
                    .unwrap_or_else(|| {
                        panic!(
                            "{}: no character display for race {} sex {}",
                            self.dir.display(),
                            player.race,
                            player.appearance.sex
                        )
                    }),
                player_stand.map_or(StandState::Stand, |stand| stand.0),
            ),
            (None, Some(display)) => (
                display.display_id,
                pose.map_or(StandState::Stand, |pose| pose.stand_state),
            ),
            // `Unit::RestoreDisplayId` of a unit spawned without a display.
            (None, None) => (0, StandState::Stand),
        };
        let Some(eye) = self.eyes.eye(display, stand_state as u8) else {
            if self.missing.lock().unwrap().insert(display) {
                // The first unit to show it: a spawn, or a creature a script summoned.
                let (display_id, dir) = (display, self.dir.display());
                let unit = npc.map_or_else(
                    || format!("{entity}"),
                    |npc| format!("{} (entry {}, {entity})", npc.name, npc.template_id),
                );
                error!(
                    "Display {display_id} in stand state {stand_state:?} has no eye row in \
                     {dir} (first shown by {unit}): units showing it are out of everyone's \
                     line of sight"
                );
            }
            return None;
        };
        let feet = Vec3::new(position.x, position.y, position.z);
        Some(Body::placed(
            feet,
            rotation.map_or(0.0, |rotation| rotation.y),
            eye,
        ))
    }

    /// The first of `target`'s eyes, chest and feet a clear ray from `watcher`'s eyes
    /// reaches on map `map`.
    pub fn witness(&self, map: InMap, watcher: &Body, target: &Body) -> Option<Vec3> {
        target
            .aim_points()
            .into_iter()
            .find(|&end| self.clear(map, watcher.eyes, end))
    }

    /// Segments tested against the bake so far (read by the tick-cost probe).
    #[cfg(test)]
    pub fn rays(&self) -> u64 {
        self.rays.load(Ordering::Relaxed)
    }

    /// Whether `watcher` sees `target` on map `map`: a clear ray from its eyes to the
    /// target's eyes, chest or feet.
    pub fn sees(&self, map: InMap, watcher: &Body, target: &Body) -> bool {
        self.witness(map, watcher, target).is_some()
    }

    pub(crate) fn set_live_hulls(&mut self, hulls: crate::game_objects::collision::LiveHulls) {
        self.live_hulls = hulls;
    }

    pub(crate) fn live_clear(&self, map: InMap, a: Vec3, b: Vec3) -> bool {
        self.live_hulls
            .get(&map)
            .into_iter()
            .flatten()
            .all(|hull| !hull.blocks(a, b))
    }

    /// Whether the segment `a..b` crosses no baked or enabled live GO triangle.
    pub fn clear(&self, map: InMap, a: Vec3, b: Vec3) -> bool {
        self.rays.fetch_add(1, Ordering::Relaxed);
        self.live_clear(map, a, b) && self.with_map(map, |tiles| tiles.clear(a, b))
    }

    /// First static or enabled dynamic collision along a movement segment.
    pub(crate) fn first_hit(&self, map: InMap, a: Vec3, b: Vec3) -> Option<f32> {
        let baked = self.with_map(map, |tiles| tiles.first_hit(a, b));
        self.live_hulls
            .get(&map)
            .into_iter()
            .flatten()
            .filter_map(|hull| hull.first_hit(a, b))
            .chain(baked)
            .min_by(f32::total_cmp)
    }

    fn with_map<T>(&self, map: InMap, query: impl FnOnce(&mut LosMap) -> Result<T, String>) -> T {
        let mut maps = self.maps.lock().unwrap();
        let tiles = maps
            .entry(map.map_id)
            .or_insert_with(|| LosMap::new(&self.dir.join(map.map_id.to_string())));
        query(tiles).unwrap_or_else(|err| panic!("LOS tile of map {}: {err}", map.map_id))
    }
}

/// Whether `watcher` sees `target` (both in `map`); true when no LOS is loaded (an app
/// without the bake, as unit tests run), false when either has no eye row.
pub(crate) fn unit_sees(
    los: Option<&LineOfSight>,
    map: InMap,
    watcher: BodyParts,
    target: BodyParts,
) -> bool {
    let Some(los) = los else {
        return true;
    };
    match (los.body(watcher), los.body(target)) {
        (Some(watcher), Some(target)) => los.sees(map, &watcher, &target),
        _ => false,
    }
}

/// The body parts of `unit` in `world`; `None` without a position.
fn parts_in_world(world: &World, unit: Entity) -> Option<BodyParts<'_>> {
    let entity = world.get_entity(unit).ok()?;
    Some((
        unit,
        entity.get::<Npc>(),
        entity.get::<Position>()?,
        entity.get::<Rotation>(),
        entity.get::<ModelDisplay>(),
        entity.get::<UnitPose>(),
        entity.get::<Player>(),
        entity.get::<PlayerStandState>(),
    ))
}

/// [`unit_sees`] for two entities of `world`.
pub(crate) fn sees_in_world(world: &World, watcher: Entity, target: Entity) -> bool {
    let Some(map) = world.get::<InMap>(watcher).copied() else {
        return true;
    };
    match (
        parts_in_world(world, watcher),
        parts_in_world(world, target),
    ) {
        (Some(watcher), Some(target)) => {
            unit_sees(world.get_resource::<LineOfSight>(), map, watcher, target)
        }
        _ => true,
    }
}

/// Area-hit visibility from the recipient to the area center. TC's point LOS
/// raises the destination by the recipient's collision height; our eye-geometry
/// contract uses its baked eye height instead (no witness from the caster).
pub(crate) fn sees_area_center(world: &World, target: Entity, center: Vec3) -> bool {
    let Some(los) = world.get_resource::<LineOfSight>() else {
        return true;
    };
    let Some(map) = world.get::<InMap>(target).copied() else {
        return false;
    };
    let Some(body) = parts_in_world(world, target).and_then(|parts| los.body(parts)) else {
        return false;
    };
    let eye_height = body.eyes.y - body.feet.y;
    los.clear(map, body.eyes, center + Vec3::Y * eye_height)
}

pub struct LineOfSightPlugin;

impl Plugin for LineOfSightPlugin {
    fn build(&self, app: &mut App) {
        // Ground errors name the missing ground files first; the LOS bake sits beside them.
        app.add_systems(
            PostStartup,
            open_line_of_sight.after(crate::ground::open_ground_map),
        );
    }
}

/// The bake output beside the ground files; the server refuses to start without its eye
/// table (run `los-bake`, docs/specs/line-of-sight.md).
fn open_line_of_sight(mut commands: Commands) {
    let dir = crate::ground::ground_data_dir().join(LOS_DIR);
    let los = LineOfSight::load(&dir)
        .unwrap_or_else(|err| panic!("No line of sight bake in {}: {err}", dir.display()));
    info!(
        "Line of sight loads from {} ({} eye rows)",
        dir.display(),
        los.eyes.len()
    );
    commands.insert_resource(los);
}
