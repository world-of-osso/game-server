//! Creature line of sight (docs/specs/line-of-sight.md, Creatures): a ray search over the
//! baked LOS tiles (`crates/los`) from the creature's eye point to the target's eyes,
//! chest and feet. LOS holds when any ray is clear.
//!
//! The bake output (`los-bake --out <ground dir>/los`) loads from the ground directory:
//! the eye table at startup, each map's tiles on first use. A map without baked tiles
//! has nothing that blocks; a unit whose display and stand state have no eye row (model
//! not extracted, or neither eye geometry nor a head bone) is not LOS-checked.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bevy::math::Vec3;
use bevy::prelude::*;
use los::LosMap;
use los::eye::EyeTable;
use shared::components::{
    ModelDisplay, Player, PlayerStandState, Position, Rotation, StandState, UnitPose,
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
}

/// Where a unit stands and sees from, in Bevy space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub feet: Vec3,
    pub eyes: Vec3,
}

impl Body {
    /// Halfway between the feet and the eyes.
    fn chest(&self) -> Vec3 {
        (self.feet + self.eyes) * 0.5
    }
}

/// What places a unit's eyes: its components.
pub(crate) type BodyParts<'a> = (
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
        })
    }

    /// The unit's feet and eyes: the eye point of its display (a player's race and sex
    /// display) in its stand state, turned to its facing. `None` without an eye row.
    pub fn body(
        &self,
        (position, rotation, display, pose, player, player_stand): BodyParts,
    ) -> Option<Body> {
        let (display, stand_state) = match (player, display) {
            (Some(player), _) => (
                self.eyes
                    .player_display(player.race, player.appearance.sex)?,
                player_stand.map_or(StandState::Stand, |stand| stand.0),
            ),
            (None, Some(display)) => (
                display.display_id,
                pose.map_or(StandState::Stand, |pose| pose.stand_state),
            ),
            (None, None) => return None,
        };
        let [forward, left, up] = self.eyes.eye(display, stand_state as u8)?;
        // `Rotation.y` is the WoW orientation plus a quarter turn (zones.rs spawn).
        let orientation = rotation.map_or(0.0, |rotation| rotation.y) - std::f32::consts::FRAC_PI_2;
        let (sin, cos) = orientation.sin_cos();
        let wow_x = forward * cos - left * sin;
        let wow_y = forward * sin + left * cos;
        let feet = Vec3::new(position.x, position.y, position.z);
        Some(Body {
            feet,
            eyes: feet + Vec3::new(wow_x, up, -wow_y),
        })
    }

    /// Whether `watcher` sees `target` on map `map`: a clear ray from its eyes to the
    /// target's eyes, chest or feet.
    pub fn sees(&self, map: InMap, watcher: &Body, target: &Body) -> bool {
        let mut maps = self.maps.lock().unwrap();
        let tiles = maps
            .entry(map.map_id)
            .or_insert_with(|| LosMap::new(&self.dir.join(map.map_id.to_string())));
        [target.eyes, target.chest(), target.feet]
            .into_iter()
            .any(|point| {
                tiles
                    .clear(watcher.eyes, point)
                    .unwrap_or_else(|err| panic!("LOS tile of map {}: {err}", map.map_id))
            })
    }
}

/// Whether `watcher` sees `target` (both in `map`); true when either has no eye row or
/// no LOS is loaded.
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
        _ => true,
    }
}

/// [`unit_sees`] for two entities of `world`.
pub(crate) fn sees_in_world(world: &World, watcher: Entity, target: Entity) -> bool {
    let Some(map) = world.get::<InMap>(watcher).copied() else {
        return true;
    };
    let parts = |unit: Entity| -> Option<BodyParts> {
        let entity = world.get_entity(unit).ok()?;
        Some((
            entity.get::<Position>()?,
            entity.get::<Rotation>(),
            entity.get::<ModelDisplay>(),
            entity.get::<UnitPose>(),
            entity.get::<Player>(),
            entity.get::<PlayerStandState>(),
        ))
    };
    match (parts(watcher), parts(target)) {
        (Some(watcher), Some(target)) => {
            unit_sees(world.get_resource::<LineOfSight>(), map, watcher, target)
        }
        _ => true,
    }
}

pub struct LineOfSightPlugin;

impl Plugin for LineOfSightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, open_line_of_sight);
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
