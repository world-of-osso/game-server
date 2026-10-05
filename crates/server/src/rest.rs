//! Rest state and rested XP, as TrinityCore `RestMgr` keeps them. Near an innkeeper the
//! player rests (`REST_FLAG_IN_TAVERN`) and its rested XP pool grows by 5% of a level per
//! 8 h (`RestMgr::Update`); logging out stores the pool and whether the player was resting,
//! and the next login adds the offline time at the resting or wilderness rate
//! (`Player::LoadFromDB`). Kills spend the pool (`RestMgr::GetRestBonusFor`,
//! `experience::apply_xp_awards`). The pool reaches the client in `PlayerXpUpdate` and
//! `RestStateUpdate`. City rest (`REST_FLAG_IN_CITY`, AreaTable `LinkedChat`) is not
//! modelled: the server has no area flags.

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Level, Position};
use shared::protocol::{RestAreaKindSnapshot, RestChannel, RestSnapshot, RestStateUpdate};
use shared::xp::RestedXp;

use crate::auth::Authenticated;
use crate::experience::{XpDirty, xp_to_next_level};
use crate::networking::PlayerLink;
use crate::persistence::{ClientId, GameDatabase};
use crate::persistence_progression::{RestData, load_rest};
use crate::world_data::WorldData;

const NPC_FLAG_INNKEEPER: u32 = 0x0001_0000;
const INNKEEPER_REST_RADIUS: f32 = 20.0;
const INNKEEPER_REST_RADIUS_SQ: f32 = INNKEEPER_REST_RADIUS * INNKEEPER_REST_RADIUS;

/// Rest "bubbles" (`RestMgr::CalcExtraPerSec`: NextLevelXP / 72000 × bubble per second).
/// 0.125 resting, in game (`RestMgr::Update`) or logged out resting (Player.cpp:18843-18846);
/// it is `RestedXp::accumulate`'s 5% of a level per 8 h.
const RESTING_BUBBLE: f32 = 0.125;
/// Logged out away from an inn or city (Player.cpp:18841).
const WILDERNESS_BUBBLE: f32 = 0.031;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestAreaKind {
    Inn,
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct PlayerRestState {
    rested_xp: RestedXp,
    area_kind: Option<RestAreaKind>,
    /// Game time and pool when the current rest began (`RestMgr::_restTime`); `None`
    /// while not resting.
    rest_start: Option<(f64, u32)>,
    last_sent: Option<RestSnapshot>,
}

impl PlayerRestState {
    pub(crate) fn rested_xp(&self) -> u32 {
        self.rested_xp.amount
    }

    /// `RestMgr::GetRestBonusFor`: kill XP gains its own amount again from the pool,
    /// at most what the pool holds. Returns the total gain.
    pub(crate) fn spend_on_kill(&mut self, xp: u32, now: f64) -> u32 {
        let (total, _) = self.rested_xp.apply_bonus(xp);
        self.restart_rest(now);
        total
    }

    fn restart_rest(&mut self, now: f64) {
        self.rest_start = self.area_kind.map(|_| (now, self.rested_xp.amount));
    }

    /// The pool after resting from `rest_start` to `now` at `xp_to_level`, capped.
    fn rested_by(&self, now: f64, xp_to_level: u32) -> Option<u32> {
        let (since, amount) = self.rest_start?;
        let mut pool = RestedXp {
            amount,
            ..self.rested_xp
        };
        pool.accumulate(((now - since) / 3600.0) as f32, xp_to_level);
        Some(pool.amount)
    }

    /// The row `Player::SaveToDB` writes at logout `logout_unix_secs`.
    pub(crate) fn logout_data(&self, logout_unix_secs: u64) -> RestData {
        RestData {
            rested_xp: self.rested_xp.amount,
            logout_unix_secs,
            logout_resting: self.area_kind.is_some(),
        }
    }
}

#[cfg(test)]
impl PlayerRestState {
    /// A player out of any rest area holding `rested_xp` at level `level`.
    pub(crate) fn fixture(level: u8, rested_xp: u32) -> Self {
        let mut pool = RestedXp::new(xp_to_next_level(level));
        pool.amount = rested_xp;
        Self {
            rested_xp: pool,
            area_kind: None,
            rest_start: None,
            last_sent: None,
        }
    }
}

pub(crate) fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after 1970")
        .as_secs()
}

#[derive(Resource, Debug, Default, Clone, PartialEq)]
struct RestAreaRegistry {
    innkeepers: Vec<Vec3>,
    initialized: bool,
}

pub struct RestPlugin;

impl Plugin for RestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RestAreaRegistry>();
        app.add_systems(
            Update,
            (
                initialize_rest_area_registry,
                update_player_rest_state,
                accumulate_rested_xp,
            )
                .chain(),
        );
    }
}

fn initialize_rest_area_registry(
    mut registry: ResMut<RestAreaRegistry>,
    world_data: Option<Res<WorldData>>,
) {
    if registry.initialized {
        return;
    }
    let Some(world_data) = world_data else { return };
    registry.innkeepers = collect_innkeeper_positions(&world_data);
    registry.initialized = true;
}

type RestPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ClientId,
        &'static Position,
        &'static Level,
        Option<&'static mut PlayerRestState>,
    ),
    Or<(Changed<Position>, Changed<Level>, Without<PlayerRestState>)>,
>;

/// What a login reads and marks: the saved rest row and the XP bar to resend.
#[derive(bevy::ecs::system::SystemParam)]
struct RestLogin<'w> {
    db: Option<Res<'w, GameDatabase>>,
    xp_dirty: Option<ResMut<'w, XpDirty>>,
}

fn update_player_rest_state(
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut players: RestPlayerQuery,
    registry: Res<RestAreaRegistry>,
    time: Res<Time>,
    mut login: RestLogin,
    mut commands: Commands,
    mut senders: Query<&mut MessageSender<RestStateUpdate>>,
) {
    if !registry.initialized {
        return;
    }
    let now = time.elapsed_secs_f64();

    for (conn_entity, link) in &connections {
        let Ok((client_id, position, level, rest_state)) = players.get_mut(link.0) else {
            continue;
        };

        let Some(snapshot) = (if let Some(rest_state) = rest_state {
            update_existing_rest_state(position, level, rest_state, &registry, now)
        } else {
            let saved = login
                .db
                .as_ref()
                .and_then(|db| load_rest(&db.db, client_id.0));
            let state = login_rest_state(position, level.0, &registry, saved, now, unix_secs());
            let snapshot = build_rest_snapshot(&state);
            commands.entity(link.0).insert(state);
            if let Some(dirty) = login.xp_dirty.as_deref_mut() {
                dirty.0.insert(client_id.0);
            }
            Some(snapshot)
        }) else {
            continue;
        };

        send_rest_update(
            conn_entity,
            snapshot,
            &mut senders,
            "rest state updated".into(),
        );
    }
}

/// `Player::LoadFromDB`: the saved pool plus the offline time since logout at
/// `RESTING_BUBBLE` when logged out resting, else `WILDERNESS_BUBBLE`.
fn login_rest_state(
    position: &Position,
    level: u8,
    registry: &RestAreaRegistry,
    saved: Option<RestData>,
    now: f64,
    now_unix_secs: u64,
) -> PlayerRestState {
    let xp_to_level = xp_to_next_level(level);
    let mut rested_xp = RestedXp::new(xp_to_level);
    if let Some(saved) = saved {
        rested_xp.amount = saved.rested_xp.min(rested_xp.max);
        let bubble = if saved.logout_resting {
            RESTING_BUBBLE
        } else {
            WILDERNESS_BUBBLE
        };
        let offline_hours = now_unix_secs.saturating_sub(saved.logout_unix_secs) as f32 / 3600.0;
        rested_xp.accumulate(offline_hours * bubble / RESTING_BUBBLE, xp_to_level);
    }
    let area_kind = detect_rest_area_kind(position, registry);
    rested_xp.in_rest_area = area_kind.is_some();
    let mut state = PlayerRestState {
        rested_xp,
        area_kind,
        rest_start: None,
        last_sent: None,
    };
    state.restart_rest(now);
    state.last_sent = Some(build_rest_snapshot(&state));
    state
}

/// Entering or leaving a rest area, or a level-up, settles the pool rested so far and
/// starts the next rest stretch (`RestMgr::SetRestFlag` / `RemoveRestFlag`).
fn update_existing_rest_state(
    position: &Position,
    level: &Level,
    mut state: Mut<PlayerRestState>,
    registry: &RestAreaRegistry,
    now: f64,
) -> Option<RestSnapshot> {
    let xp_to_level = xp_to_next_level(level.0);
    let area_kind = detect_rest_area_kind(position, registry);
    let max_before = state.rested_xp.max;
    let mut pool = state.rested_xp;
    pool.update_max(xp_to_level);
    if area_kind != state.area_kind || pool.max != max_before {
        if let Some(amount) = state.rested_by(now, xp_to_level) {
            state.rested_xp.amount = amount;
        }
        state.rested_xp.update_max(xp_to_level);
        state.area_kind = area_kind;
        state.rested_xp.in_rest_area = area_kind.is_some();
        state.restart_rest(now);
    }

    let snapshot = build_rest_snapshot(&state);
    if state.last_sent.as_ref() == Some(&snapshot) {
        return None;
    }
    state.last_sent = Some(snapshot.clone());
    Some(snapshot)
}

/// `Player::Update` -> `RestMgr::Update` (Player.cpp:1015-1016): while resting the pool
/// grows by `RESTING_BUBBLE`; each whole XP gained reaches the client.
fn accumulate_rested_xp(
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut players: Query<(&ClientId, &Level, &mut PlayerRestState)>,
    time: Res<Time>,
    mut xp_dirty: Option<ResMut<XpDirty>>,
    mut senders: Query<&mut MessageSender<RestStateUpdate>>,
) {
    let now = time.elapsed_secs_f64();
    for (conn_entity, link) in &connections {
        let Ok((client_id, level, mut state)) = players.get_mut(link.0) else {
            continue;
        };
        let Some(amount) = state.rested_by(now, xp_to_next_level(level.0)) else {
            continue;
        };
        if amount == state.rested_xp.amount {
            continue;
        }
        state.rested_xp.amount = amount;
        let snapshot = build_rest_snapshot(&state);
        state.last_sent = Some(snapshot.clone());
        send_rest_update(conn_entity, snapshot, &mut senders, "rested xp".into());
        if let Some(dirty) = xp_dirty.as_deref_mut() {
            dirty.0.insert(client_id.0);
        }
    }
}

fn detect_rest_area_kind(position: &Position, registry: &RestAreaRegistry) -> Option<RestAreaKind> {
    is_near_innkeeper(position, &registry.innkeepers).then_some(RestAreaKind::Inn)
}

fn is_near_innkeeper(position: &Position, innkeepers: &[Vec3]) -> bool {
    innkeepers.iter().any(|innkeeper| {
        let dx = position.x - innkeeper.x;
        let dy = position.y - innkeeper.y;
        let dz = position.z - innkeeper.z;
        dx * dx + dy * dy + dz * dz <= INNKEEPER_REST_RADIUS_SQ
    })
}

fn collect_innkeeper_positions(world_data: &WorldData) -> Vec<Vec3> {
    let mut innkeepers = Vec::new();
    for spawn in &world_data.spawns {
        let Some(template) = world_data.templates.get(&spawn.template_entry) else {
            continue;
        };
        if template.npc_flags & NPC_FLAG_INNKEEPER == 0 {
            continue;
        }
        let (x, y, z) = crate::terrain_height::wow_to_bevy(
            spawn.position_x,
            spawn.position_y,
            spawn.position_z,
        );
        innkeepers.push(Vec3::new(x, y, z));
    }
    innkeepers
}

/// Admin changes restart the accumulation baseline and publish both owner snapshots.
/// Offline characters have no live state; the handler has already saved their pool.
pub(crate) fn set_live_rested(world: &mut World, character_id: u64, amount: u32) {
    let mut players = world.query_filtered::<(Entity, &ClientId), With<PlayerRestState>>();
    let Some(player) = players
        .iter(world)
        .find_map(|(entity, id)| (id.0 == character_id).then_some(entity))
    else {
        return;
    };
    let now = world.resource::<Time>().elapsed_secs_f64();
    let mut state = world
        .get_mut::<PlayerRestState>(player)
        .expect("queried rest state");
    state.rested_xp.amount = amount.min(state.rested_xp.max);
    state.restart_rest(now);
    let snapshot = build_rest_snapshot(&state);
    state.last_sent = Some(snapshot.clone());
    let saved = state.logout_data(unix_secs());
    let db = &world.resource::<GameDatabase>().db;
    crate::persistence_progression::save_rest(db, character_id, &saved);
    world.resource_mut::<XpDirty>().0.insert(character_id);
    let mut connections = world.query_filtered::<(Entity, &PlayerLink), With<Authenticated>>();
    let connection = connections
        .iter(world)
        .find_map(|(entity, link)| (link.0 == player).then_some(entity));
    if let Some(mut sender) =
        connection.and_then(|entity| world.get_mut::<MessageSender<RestStateUpdate>>(entity))
    {
        sender.send::<RestChannel>(RestStateUpdate {
            snapshot: Some(snapshot),
            message: Some("rested xp".into()),
            error: None,
        });
    }
}

fn build_rest_snapshot(state: &PlayerRestState) -> RestSnapshot {
    RestSnapshot {
        in_rest_area: state.rested_xp.in_rest_area,
        rest_area_kind: state.area_kind.map(map_rest_area_kind_snapshot),
        rested_xp: state.rested_xp.amount,
        rested_xp_max: state.rested_xp.max,
    }
}

fn map_rest_area_kind_snapshot(kind: RestAreaKind) -> RestAreaKindSnapshot {
    match kind {
        RestAreaKind::Inn => RestAreaKindSnapshot::Inn,
    }
}

fn send_rest_update(
    conn_entity: Entity,
    snapshot: RestSnapshot,
    senders: &mut Query<&mut MessageSender<RestStateUpdate>>,
    message: String,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<RestChannel>(RestStateUpdate {
        snapshot: Some(snapshot),
        message: Some(message),
        error: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence_progression::save_rest;
    use crate::world_data::{CreatureModelInfo, CreatureSpawn, CreatureTemplate};
    use bevy::ecs::change_detection::DetectChanges;
    use std::collections::HashMap;

    fn rest_template(npc_flags: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry: 1,
            name: "Fixture innkeeper".into(),
            subname: String::new(),
            creature_type: 7,
            family: 0,
            type_flags: 0,
            min_level: 1,
            max_level: 1,
            faction: 0,
            npc_flags,
            speed_walk: 1.0,
            speed_run: 1.0,
            health_modifier: 1.0,
            unit_class: 1,
            expansion: 0,
            rank: 0,
            experience_modifier: 1.0,
            gives_experience: true,
            script_name: String::new(),
            detection_range: 20.0,
            aggro_mode: crate::creature_aggro::AggroMode::Aggressive,
            trigger: false,
            unit_flags: 0,
            models: vec![CreatureModelInfo {
                display_id: 1,
                probability: 1.0,
                is_trigger: false,
            }],
            tuning: None,
            armor: 0.0,
            mana_modifier: 1.0,
            regenerates_power: false,
            regenerates_health: true,
        }
    }

    fn rest_spawn(position_x: f32, position_y: f32) -> CreatureSpawn {
        CreatureSpawn {
            guid: 1,
            template_entry: 1,
            map: 0,
            zone_id: 12,
            position_x,
            position_y,
            position_z: 56.0,
            orientation: 0.0,
            wander_distance: 0.0,
            movement_type: 0,
            spawntimesecs: 120,
            display_id: None,
            unit_flags: None,
            spawn_difficulties: vec![0],
            equipment_id: 0,
        }
    }

    fn world_data_with_npc_flags(npc_flags: u32, position_x: f32, position_y: f32) -> WorldData {
        let mut templates = HashMap::new();
        templates.insert(1, rest_template(npc_flags));
        WorldData {
            templates,
            spawns: vec![rest_spawn(position_x, position_y)],
            spawns_by_map: HashMap::from([(0, vec![0])]),
            waypoints: HashMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn unchanged_rest_update_does_not_dirty_state_and_transition_updates_area() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let registry = RestAreaRegistry {
            innkeepers: vec![Vec3::ZERO],
            initialized: true,
        };
        app.insert_resource(registry);

        let player = app
            .world_mut()
            .spawn((
                Position {
                    x: 50.0,
                    y: 0.0,
                    z: 0.0,
                },
                ClientId(1),
                Level(1),
                PlayerRestState {
                    rested_xp: RestedXp::new(xp_to_next_level(1)),
                    area_kind: None,
                    rest_start: None,
                    last_sent: Some(RestSnapshot {
                        in_rest_area: false,
                        rest_area_kind: None,
                        rested_xp: 0,
                        rested_xp_max: xp_to_next_level(1) * 3 / 2,
                    }),
                },
            ))
            .id();
        app.world_mut()
            .spawn((Authenticated { account_id: 1 }, PlayerLink(player)));

        app.add_systems(Update, update_player_rest_state);
        app.update();

        let before = app
            .world()
            .entity(player)
            .get_ref::<PlayerRestState>()
            .unwrap()
            .last_changed();
        app.update();
        let after = app
            .world()
            .entity(player)
            .get_ref::<PlayerRestState>()
            .unwrap()
            .last_changed();

        assert_eq!(after, before);

        app.world_mut()
            .entity_mut(player)
            .get_mut::<Position>()
            .unwrap()
            .x = 0.0;
        app.update();

        let state = app.world().entity(player).get::<PlayerRestState>().unwrap();
        assert_eq!(state.area_kind, Some(RestAreaKind::Inn));
        assert!(state.rested_xp.in_rest_area);
    }

    /// A level 10 player (10 590 XP to level) at an innkeeper, on a clock that advances
    /// 80 h per frame.
    fn resting_app(db: Option<GameDatabase>) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(80 * 3600),
        ));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_secs(1000 * 3600));
        // One fixed step per 80 h frame instead of 64 a second.
        app.insert_resource(Time::<Fixed>::from_seconds(80.0 * 3600.0));
        app.insert_resource(RestAreaRegistry {
            innkeepers: vec![Vec3::ZERO],
            initialized: true,
        });
        if let Some(db) = db {
            app.insert_resource(db);
        }
        let player = app
            .world_mut()
            .spawn((
                ClientId(7),
                Position {
                    x: 5.0,
                    y: 0.0,
                    z: 0.0,
                },
                Level(10),
            ))
            .id();
        app.world_mut()
            .spawn((Authenticated { account_id: 1 }, PlayerLink(player)));
        app.add_systems(
            Update,
            (update_player_rest_state, accumulate_rested_xp).chain(),
        );
        (app, player)
    }

    fn rested(app: &App, player: Entity) -> u32 {
        app.world()
            .get::<PlayerRestState>(player)
            .unwrap()
            .rested_xp()
    }

    /// `RestMgr::Update`: 5% of a level per 8 h resting, so 80 h give half of level 10's
    /// 10 590 XP; the pool stops at 1.5 levels (15 885).
    #[test]
    fn resting_at_an_inn_gains_rested_xp_over_time_up_to_one_and_a_half_levels() {
        let (mut app, player) = resting_app(None);
        app.update();
        assert_eq!(rested(&app, player), 0);

        app.update();
        assert_eq!(rested(&app, player), 5_295);

        app.update();
        app.update();
        assert_eq!(rested(&app, player), 15_885);
    }

    /// `Player::LoadFromDB`: logged out resting, the 80 h offline count at the resting
    /// rate on the next login.
    #[test]
    fn logging_out_at_an_inn_and_back_in_80_hours_later_grants_the_rested_bonus() {
        let db = crate::persistence::temp_database();
        let state = PlayerRestState {
            rested_xp: RestedXp::new(xp_to_next_level(10)),
            area_kind: Some(RestAreaKind::Inn),
            rest_start: None,
            last_sent: None,
        };
        save_rest(&db, 7, &state.logout_data(unix_secs() - 80 * 3600));

        let (mut app, player) = resting_app(Some(GameDatabase { db }));
        app.update();

        assert_eq!(rested(&app, player), 5_295);
    }

    #[test]
    fn collect_innkeeper_positions_filters_by_npc_flag() {
        let world_data = world_data_with_npc_flags(NPC_FLAG_INNKEEPER, -9462.66, 16.19);
        let innkeepers = collect_innkeeper_positions(&world_data);
        assert_eq!(innkeepers.len(), 1);

        let world_data = world_data_with_npc_flags(0, -9462.66, 16.19);
        let innkeepers = collect_innkeeper_positions(&world_data);
        assert!(innkeepers.is_empty());
    }

    #[test]
    fn detect_rest_area_kind_triggers_near_innkeeper() {
        let world_data = world_data_with_npc_flags(NPC_FLAG_INNKEEPER, -9462.66, 16.19);
        let registry = RestAreaRegistry {
            innkeepers: collect_innkeeper_positions(&world_data),
            initialized: true,
        };
        let (x, y, z) = crate::terrain_height::wow_to_bevy(-9462.66, 16.19, 56.0);
        let close = Position { x, y, z };
        let far = Position { x: x + 50.0, y, z };

        assert_eq!(
            detect_rest_area_kind(&close, &registry),
            Some(RestAreaKind::Inn)
        );
        assert_eq!(detect_rest_area_kind(&far, &registry), None);
    }

    #[test]
    fn build_rest_snapshot_marks_inn_state() {
        let mut state = PlayerRestState {
            rested_xp: RestedXp::new(xp_to_next_level(1)),
            area_kind: None,
            rest_start: None,
            last_sent: None,
        };
        state.area_kind = Some(RestAreaKind::Inn);
        state.rested_xp.in_rest_area = true;

        let snapshot = build_rest_snapshot(&state);
        assert!(snapshot.in_rest_area);
        assert_eq!(snapshot.rest_area_kind, Some(RestAreaKindSnapshot::Inn));
        assert_eq!(snapshot.rested_xp_max, xp_to_next_level(1) * 3 / 2);
    }
}
