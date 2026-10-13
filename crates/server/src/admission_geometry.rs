//! Off-thread geometry preparation; admission has no deadline or unready bypass.
use crate::auth::{Authenticated, InWorld, PendingForcedDisconnect};
use crate::ground::{
    GroundMap,
    preparation::{GroundPreparation, PreparedGround},
};
use crate::line_of_sight::LineOfSight;
use crate::maps::InMap;
use crate::navigation::Navigation;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use shared::components::{Npc, Position};
use shared::protocol::SelectCharacter;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};
use std::thread::JoinHandle;

type Tile = (u32, u32);

#[derive(Component)]
pub(crate) struct PendingCharacterSelection {
    pub character_id: u64,
    pub account_id: u64,
    pub map_id: u32,
}

#[derive(SystemParam)]
pub(crate) struct AdmissionGeometry<'w, 's> {
    queue: Local<'s, AdmissionQueue>,
    ground: Option<ResMut<'w, GroundMap>>,
    los: Option<ResMut<'w, LineOfSight>>,
    navigation: Option<ResMut<'w, Navigation>>,
    pending: Query<
        'w,
        's,
        (
            Entity,
            &'static Authenticated,
            &'static PendingCharacterSelection,
        ),
        (Without<InWorld>, Without<PendingForcedDisconnect>),
    >,
    neighbours: AdmissionNeighbours<'w, 's>,
}

impl AdmissionGeometry<'_, '_> {
    pub fn poll(&mut self) {
        self.queue.receive_prepared();
        self.queue.install_one_step(
            self.ground.as_deref_mut(),
            self.los.as_deref_mut(),
            self.navigation.as_deref_mut(),
        );
    }

    pub fn completed_requests(&self) -> Vec<(Entity, u64, SelectCharacter)> {
        self.pending
            .iter()
            .filter(|(_, auth, pending)| {
                auth.account_id == pending.account_id
                    && !self.queue.pending.contains_key(&pending.map_id)
            })
            .map(|(connection, auth, pending)| {
                (
                    connection,
                    auth.account_id,
                    SelectCharacter {
                        character_id: pending.character_id,
                    },
                )
            })
            .collect()
    }

    pub fn ensure_ready(&mut self, map: InMap, position: Position) -> Result<bool, String> {
        // Fixtures without asset resources have no geometry work to perform. The
        // production Ground/LOS/Navigation plugins install all three resources.
        if self.ground.is_none() && self.los.is_none() && self.navigation.is_none() {
            return Ok(true);
        }
        let tiles = self.neighbours.tiles(map, position);
        let needed: Vec<_> = tiles
            .into_iter()
            .filter(|tile| !self.queue.ready.contains(&(map.map_id, *tile)))
            .collect();
        if needed.is_empty() {
            return Ok(true);
        }
        if let Some(error) = needed
            .iter()
            .find_map(|tile| self.queue.failed.get(&(map.map_id, *tile)))
        {
            return Err(error.clone());
        }
        if self.queue.pending.contains_key(&map.map_id) {
            return Ok(false);
        }
        let request = PreparationRequest {
            map_id: map.map_id,
            tiles: needed,
            ground: self
                .ground
                .as_ref()
                .map(|ground| ground.preparation(map.map_id)),
            los: self
                .los
                .as_ref()
                .map(|los| los.preparation_directory(map.map_id)),
            navigation: self
                .navigation
                .as_ref()
                .map(|nav| nav.preparation_directory(map.map_id)),
        };
        self.queue.submit(request)?;
        Ok(false)
    }
}

struct PreparationRequest {
    map_id: u32,
    tiles: Vec<Tile>,
    ground: Option<GroundPreparation>,
    los: Option<PathBuf>,
    navigation: Option<PathBuf>,
}

struct PreparedRegion {
    ground: Option<PreparedGround>,
    los: Option<los::LosMap>,
    navigation: Option<PreparedNavigation>,
}

struct PreparedNavigation {
    baked: bool,
    tiles: VecDeque<navigation::PreparedNavigationTile>,
}

type PreparedReply = (u32, Result<PreparedRegion, String>);

struct AdmissionQueue {
    sender: Option<mpsc::Sender<PreparationRequest>>,
    receiver: Mutex<mpsc::Receiver<PreparedReply>>,
    worker: Option<JoinHandle<()>>,
    pending: HashMap<u32, Vec<Tile>>,
    ready: HashSet<(u32, Tile)>,
    failed: HashMap<(u32, Tile), String>,
    installing: VecDeque<(u32, PreparedRegion)>,
    worker_failed: bool,
}

impl Default for AdmissionQueue {
    fn default() -> Self {
        let (sender, work) = mpsc::channel::<PreparationRequest>();
        let (finished, receiver) = mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("admission-geometry".into())
            .spawn(move || {
                while let Ok(request) = work.recv() {
                    let map = request.map_id;
                    let result = load_region(request);
                    if finished.send((map, result)).is_err() {
                        break;
                    }
                }
            })
            .expect("start admission geometry worker");
        Self {
            sender: Some(sender),
            receiver: Mutex::new(receiver),
            worker: Some(worker),
            pending: HashMap::new(),
            ready: HashSet::new(),
            failed: HashMap::new(),
            installing: VecDeque::new(),
            worker_failed: false,
        }
    }
}

impl Drop for AdmissionQueue {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            error!("Admission geometry worker panicked during shutdown");
        }
    }
}

impl AdmissionQueue {
    fn submit(&mut self, request: PreparationRequest) -> Result<(), String> {
        if self.worker_failed {
            return Err("geometry worker unavailable".into());
        }
        let map_id = request.map_id;
        info!(
            map_id,
            tiles = request.tiles.len(),
            "Preparing admission geometry off the tick thread"
        );
        self.pending.insert(map_id, request.tiles.clone());
        self.sender
            .as_ref()
            .expect("geometry sender")
            .send(request)
            .map_err(|_| "geometry worker disconnected".to_owned())
    }

    fn receive_prepared(&mut self) {
        loop {
            let reply = self
                .receiver
                .lock()
                .expect("geometry replies lock")
                .try_recv();
            match reply {
                Ok((map, Ok(region))) => self.installing.push_back((map, region)),
                Ok((map, Err(error))) => self.fail(map, error),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if !self.worker_failed {
                        self.worker_failed = true;
                        let maps: Vec<_> = self.pending.keys().copied().collect();
                        for map in maps {
                            self.fail(map, "geometry worker disconnected".into());
                        }
                    }
                    break;
                }
            }
        }
    }

    fn fail(&mut self, map: u32, error: String) {
        error!(map_id = map, %error, "Admission geometry load failed");
        for tile in self.pending.remove(&map).expect("failed geometry request") {
            self.failed.insert((map, tile), error.clone());
        }
    }

    fn install_one_step(
        &mut self,
        ground: Option<&mut GroundMap>,
        los: Option<&mut LineOfSight>,
        navigation: Option<&mut Navigation>,
    ) {
        let Some((map_id, region)) = self.installing.front_mut() else {
            return;
        };
        if let Some(prepared) = region.ground.take() {
            ground
                .expect("ground resource persists during preparation")
                .install_prepared(prepared);
        }
        if let Some(prepared) = region.los.take() {
            los.expect("LOS resource persists during preparation")
                .install_prepared(*map_id, prepared);
        }
        let result = install_navigation_step(*map_id, &mut region.navigation, navigation);
        if let Err(error) = result {
            let map = *map_id;
            self.installing.pop_front();
            self.fail(map, error);
            return;
        }
        if region.navigation.is_none() {
            let map = *map_id;
            self.installing.pop_front();
            for tile in self
                .pending
                .remove(&map)
                .expect("prepared geometry request")
            {
                self.ready.insert((map, tile));
            }
            info!(map_id = map, "Admission geometry ready");
        }
    }
}

fn install_navigation_step(
    map: u32,
    prepared: &mut Option<PreparedNavigation>,
    navigation: Option<&mut Navigation>,
) -> Result<(), String> {
    let Some(region) = prepared else {
        return Ok(());
    };
    let navigation = navigation.expect("navigation resource persists during preparation");
    let tile = region.tiles.pop_front();
    navigation.install_prepared(map, region.baked, tile)?;
    if region.tiles.is_empty() {
        *prepared = None;
    }
    Ok(())
}

fn load_region(request: PreparationRequest) -> Result<PreparedRegion, String> {
    let started = std::time::Instant::now();
    let ground = request
        .ground
        .map(|source| source.load(&request.tiles))
        .transpose()?;
    let los = request
        .los
        .map(|dir| {
            let mut map = los::LosMap::new(&dir);
            map.preload_tiles(request.tiles.iter().copied())?;
            Ok::<_, String>(map)
        })
        .transpose()?;
    let navigation = request
        .navigation
        .map(|dir| prepare_navigation(&dir, &request.tiles))
        .transpose()?;
    info!(
        map_id = request.map_id,
        tiles = request.tiles.len(),
        elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
        "Loaded admission geometry on worker"
    );
    Ok(PreparedRegion {
        ground,
        los,
        navigation,
    })
}

fn prepare_navigation(dir: &Path, tiles: &[Tile]) -> Result<PreparedNavigation, String> {
    let baked = match std::fs::read_dir(dir) {
        Ok(entries) => {
            let mut baked = false;
            for entry in entries {
                baked |= entry
                    .map_err(|error| format!("{}: {error}", dir.display()))?
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "nav");
            }
            baked
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(format!("{}: {error}", dir.display())),
    };
    let prepared = if baked {
        tiles
            .iter()
            .map(|&tile| navigation::PreparedNavigationTile::read(dir, tile))
            .collect::<Result<_, _>>()?
    } else {
        VecDeque::new()
    };
    Ok(PreparedNavigation {
        baked,
        tiles: prepared,
    })
}

#[derive(SystemParam)]
struct AdmissionNeighbours<'w, 's> {
    index: Option<Res<'w, crate::grid_activity::GridIndex>>,
    positions: Query<'w, 's, &'static Position, With<Npc>>,
    paths: Query<'w, 's, &'static crate::creature_ai::WaypointState>,
    formations: Query<'w, 's, &'static crate::creature_ai::FormationMember>,
    data: Option<Res<'w, crate::world_data::WorldData>>,
}

impl AdmissionNeighbours<'_, '_> {
    fn tiles(&self, map: InMap, position: Position) -> BTreeSet<Tile> {
        let point = Vec3::new(position.x, position.y, position.z);
        let mut tiles = BTreeSet::new();
        extend_tiles(&mut tiles, point, point, 2);
        let entities = self
            .index
            .as_ref()
            .map(|index| index.preparation_entities(map, &position))
            .unwrap_or_default();
        let mut paths = BTreeSet::new();
        for entity in &entities {
            if let Ok(position) = self.positions.get(*entity) {
                let point = Vec3::new(position.x, position.y, position.z);
                extend_tiles(&mut tiles, point, point, 1);
            }
            if let Ok(path) = self.paths.get(*entity) {
                paths.insert(path.path_id);
            }
            if let Ok(member) = self.formations.get(*entity)
                && let Ok(path) = self.paths.get(member.leader)
            {
                paths.insert(path.path_id);
            }
        }
        if entities.is_empty() {
            self.extend_spawn_tiles(map.map_id, &mut tiles);
        }
        for path in paths {
            self.extend_path_tiles(path, &mut tiles);
        }
        tiles
    }

    fn extend_spawn_tiles(&self, map: u32, tiles: &mut BTreeSet<Tile>) {
        let Some(data) = &self.data else { return };
        let Ok(map) = u16::try_from(map) else { return };
        for &index in data.spawns_by_map.get(&map).into_iter().flatten() {
            let spawn = &data.spawns[index];
            let (x, y, z) = crate::terrain_height::wow_to_bevy(
                spawn.position_x,
                spawn.position_y,
                spawn.position_z,
            );
            let point = Vec3::new(x, y, z);
            extend_tiles(tiles, point, point, 1);
            if spawn.movement_type == 2 {
                let addon = data
                    .addons
                    .get(&spawn.guid)
                    .or_else(|| data.template_addons.get(&spawn.template_entry));
                if let Some(addon) = addon {
                    self.extend_path_tiles(addon.path_id, tiles);
                }
            }
        }
    }

    fn extend_path_tiles(&self, path: u32, tiles: &mut BTreeSet<Tile>) {
        let Some(path) = self
            .data
            .as_ref()
            .and_then(|data| data.waypoints.get(&path))
        else {
            return;
        };
        for (index, point) in path.points.iter().enumerate() {
            let next = &path.points[(index + 1) % path.points.len()];
            let convert = |point: &crate::world_data::WaypointPoint| {
                let (x, y, z) = crate::terrain_height::wow_to_bevy(
                    point.position_x,
                    point.position_y,
                    point.position_z,
                );
                Vec3::new(x, y, z)
            };
            extend_tiles(tiles, convert(point), convert(next), 1);
        }
    }
}

fn extend_tiles(tiles: &mut BTreeSet<Tile>, from: Vec3, to: Vec3, ring: u32) {
    let a = shared::los::tile_of(from.x, from.z);
    let b = shared::los::tile_of(to.x, to.z);
    for row in a.0.min(b.0).saturating_sub(ring)..=(a.0.max(b.0) + ring).min(63) {
        for col in a.1.min(b.1).saturating_sub(ring)..=(a.1.max(b.1) + ring).min(63) {
            tiles.insert((row, col));
        }
    }
}
