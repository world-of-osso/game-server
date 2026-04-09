use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::{Health, Level, Mana, Player, Position};
use shared::death::{
    Corpse, DeathState, Graveyard, GraveyardRegistry, SpiritHealerContext, accept_spirit_healer,
    in_corpse_range, on_death, release_spirit, resurrect_at_corpse,
};
use shared::protocol::{
    AcceptSpiritHealerResurrection, DeathChannel, DeathPositionSnapshot, DeathSnapshot,
    DeathStateSnapshot, DeathStateUpdate, QueryDeathStatus, ReleaseSpirit, ResurrectAtCorpse,
    UseStuckEscape,
};

use crate::auth::{Authenticated, InWorld};
use crate::durability::EquippedDurability;
use crate::networking::PlayerLink;
use crate::world_data::WorldData;

const WORLD_MAP_ID: u16 = 0;
const GHOST_RESURRECT_HEALTH_PCT: f32 = 0.5;
const GHOST_RESURRECT_MANA_PCT: f32 = 0.5;

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerDeathState {
    pub state: DeathState,
    pub corpse: Option<Corpse>,
    pub graveyard: Option<Graveyard>,
}

impl Default for PlayerDeathState {
    fn default() -> Self {
        Self {
            state: DeathState::Alive,
            corpse: None,
            graveyard: None,
        }
    }
}

#[derive(Resource, Default)]
struct DeathServerState {
    graveyards: GraveyardRegistry,
}

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeathServerState>();
        app.add_systems(PostStartup, load_graveyards_from_world_data);
        app.add_systems(Update, send_initial_death_snapshot);
        app.add_systems(Update, process_death_queries);
        app.add_systems(Update, process_release_spirit_requests);
        app.add_systems(Update, process_resurrect_at_corpse_requests);
        app.add_systems(Update, process_spirit_healer_requests);
        app.add_systems(Update, process_stuck_escape_requests);
        app.add_systems(
            FixedUpdate,
            detect_player_deaths.after(crate::networking::apply_terrain_gravity),
        );
    }
}

fn load_graveyards_from_world_data(
    world_data: Option<Res<WorldData>>,
    mut state: ResMut<DeathServerState>,
) {
    let Some(world_data) = world_data else { return };
    state.graveyards = build_graveyard_registry(&world_data);
}

fn build_graveyard_registry(world_data: &WorldData) -> GraveyardRegistry {
    let mut graveyards = GraveyardRegistry::new();
    for spawn in &world_data.spawns {
        let Some(template) = world_data.templates.get(&spawn.template_entry) else {
            continue;
        };
        if template.npc_flags & shared::death::NPC_FLAG_SPIRIT_HEALER == 0 {
            continue;
        }
        let (x, y, z) = crate::terrain_height::wow_to_bevy(
            spawn.position_x,
            spawn.position_y,
            spawn.position_z,
        );
        graveyards.add(Graveyard {
            id: spawn.guid,
            zone_id: spawn.zone_id as u32,
            faction: 0,
            map_id: spawn.map,
            x,
            y,
            z,
        });
    }
    graveyards
}

type NewInWorldLink<'w, 's> =
    Query<'w, 's, (Entity, &'static PlayerLink), (With<Authenticated>, Added<InWorld>)>;

fn send_initial_death_snapshot(
    connections: NewInWorldLink,
    players: Query<(&Position, Option<&PlayerDeathState>)>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link) in &connections {
        let Ok((position, death_state)) = players.get(link.0) else {
            send_error(
                conn_entity,
                &mut senders,
                "death state unavailable for player",
            );
            continue;
        };
        send_update(
            conn_entity,
            position,
            death_state.cloned().unwrap_or_default(),
            Some("death state loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_death_queries(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueryDeathStatus>),
        With<Authenticated>,
    >,
    players: Query<(&Position, Option<&PlayerDeathState>)>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok((position, death_state)) = players.get(link.0) else {
            send_error(
                conn_entity,
                &mut senders,
                "death state unavailable for player",
            );
            continue;
        };
        send_update(
            conn_entity,
            position,
            death_state.cloned().unwrap_or_default(),
            Some("death state refreshed".into()),
            None,
            &mut senders,
        );
    }
}

fn process_release_spirit_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<ReleaseSpirit>),
        With<Authenticated>,
    >,
    mut players: Query<(&mut Position, &mut PlayerDeathState)>,
    state: Res<DeathServerState>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, mut death_state)) = players.get_mut(link.0) else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            match release_spirit_for_player(&mut position, &mut death_state, &state.graveyards) {
                Ok(message) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.clone(),
                        Some(message),
                        None,
                        &mut senders,
                    );
                }
                Err(error) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.clone(),
                        None,
                        Some(error),
                        &mut senders,
                    );
                }
            }
        }
    }
}

type ResurrectReceiver<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        Option<&'static Player>,
        &'static mut MessageReceiver<ResurrectAtCorpse>,
    ),
    With<Authenticated>,
>;

fn process_resurrect_at_corpse_requests(
    mut receivers: ResurrectReceiver,
    mut players: Query<(
        &mut Position,
        &mut Health,
        Option<&mut Mana>,
        &mut PlayerDeathState,
    )>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, player, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, mut health, mana, mut death_state)) = players.get_mut(link.0)
            else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            match resurrect_player_at_corpse(&mut position, &mut health, mana, &mut death_state) {
                Ok(message) => {
                    let name = player.map(|p| p.name.as_str()).unwrap_or("player");
                    send_update(
                        conn_entity,
                        &position,
                        death_state.clone(),
                        Some(format!("{name} {message}")),
                        None,
                        &mut senders,
                    );
                }
                Err(error) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.clone(),
                        None,
                        Some(error),
                        &mut senders,
                    );
                }
            }
        }
    }
}

type SpiritHealerPlayerData<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Position,
        &'static mut Health,
        Option<&'static mut Mana>,
        Option<&'static Level>,
        Option<&'static mut EquippedDurability>,
        &'static mut PlayerDeathState,
    ),
>;

fn process_spirit_healer_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &mut MessageReceiver<AcceptSpiritHealerResurrection>,
        ),
        With<Authenticated>,
    >,
    mut players: SpiritHealerPlayerData,
    time: Res<Time>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    let now = current_seconds(&time);
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, mut health, mana, level, durability, mut death_state)) =
                players.get_mut(link.0)
            else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            match resurrect_at_spirit_healer(
                &mut position,
                &mut health,
                mana,
                durability,
                &mut death_state,
                level.map(|level| level.0).unwrap_or_default(),
                now,
            ) {
                Ok(message) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.clone(),
                        Some(message),
                        None,
                        &mut senders,
                    );
                }
                Err(error) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.clone(),
                        None,
                        Some(error),
                        &mut senders,
                    );
                }
            }
        }
    }
}

fn process_stuck_escape_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<UseStuckEscape>),
        With<Authenticated>,
    >,
    mut players: Query<(&mut Position, Option<&PlayerDeathState>)>,
    state: Res<DeathServerState>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, death_state)) = players.get_mut(link.0) else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            match use_stuck_escape(&mut position, death_state, &state.graveyards) {
                Ok(message) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.cloned().unwrap_or_default(),
                        Some(message),
                        None,
                        &mut senders,
                    );
                }
                Err(error) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.cloned().unwrap_or_default(),
                        None,
                        Some(error),
                        &mut senders,
                    );
                }
            }
        }
    }
}

type DeathDetectionQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Position,
        &'static mut Health,
        Option<&'static mut EquippedDurability>,
        Option<&'static mut PlayerDeathState>,
        &'static crate::persistence::ClientId,
    ),
    With<Player>,
>;

fn detect_player_deaths(
    mut players: DeathDetectionQuery,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    time: Res<Time>,
    mut commands: Commands,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    let now = current_seconds(&time);
    for (player_entity, position, mut health, durability, death_state, client_id) in &mut players {
        if health.current > 0.0 {
            continue;
        }
        if death_state
            .as_ref()
            .is_some_and(|state| state.state != DeathState::Alive)
        {
            continue;
        }
        health.current = 0.0;
        let result = on_death(
            client_id.0,
            WORLD_MAP_ID,
            position.x,
            position.y,
            position.z,
            now,
        );
        let death_state = PlayerDeathState {
            state: DeathState::Dead,
            corpse: Some(result.corpse),
            graveyard: None,
        };
        if let Some(mut durability) = durability {
            crate::durability::apply_death_loss(&mut durability);
        }
        commands.entity(player_entity).insert(death_state.clone());
        if let Some(conn_entity) = connection_for_player(player_entity, &connections) {
            send_update(
                conn_entity,
                position,
                death_state,
                Some("you died".into()),
                None,
                &mut senders,
            );
        }
    }
}

fn connection_for_player(
    player_entity: Entity,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
) -> Option<Entity> {
    connections
        .iter()
        .find_map(|(conn_entity, link)| (link.0 == player_entity).then_some(conn_entity))
}

fn release_spirit_for_player(
    position: &mut Position,
    death_state: &mut PlayerDeathState,
    graveyards: &GraveyardRegistry,
) -> Result<String, String> {
    let corpse = death_state
        .corpse
        .as_ref()
        .ok_or_else(|| "corpse is unavailable".to_string())?;
    let result = release_spirit(
        death_state.state,
        corpse.map_id,
        corpse.x,
        corpse.y,
        corpse.z,
        0,
        graveyards,
    )
    .map_err(|_| "unable to release spirit here".to_string())?;
    death_state.state = DeathState::Ghost;
    death_state.graveyard = Some(Graveyard {
        id: result.graveyard_id,
        zone_id: 0,
        faction: 0,
        map_id: result.map_id,
        x: result.x,
        y: result.y,
        z: result.z,
    });
    position.x = result.x;
    position.y = result.y;
    position.z = result.z;
    Ok("released spirit".into())
}

fn use_stuck_escape(
    position: &mut Position,
    death_state: Option<&PlayerDeathState>,
    graveyards: &GraveyardRegistry,
) -> Result<String, String> {
    if death_state
        .as_ref()
        .is_some_and(|state| state.state != DeathState::Alive)
    {
        return Err("stuck escape is unavailable while dead".into());
    }
    let graveyard = graveyards
        .nearest(WORLD_MAP_ID, position.x, position.y, position.z, 0)
        .ok_or_else(|| "unable to find a safe graveyard".to_string())?;
    position.x = graveyard.x;
    position.y = graveyard.y;
    position.z = graveyard.z;
    Ok("stuck escape used".into())
}

fn resurrect_player_at_corpse(
    position: &mut Position,
    health: &mut Health,
    mana: Option<Mut<Mana>>,
    death_state: &mut PlayerDeathState,
) -> Result<String, String> {
    let corpse = death_state
        .corpse
        .as_ref()
        .ok_or_else(|| "corpse is unavailable".to_string())?;
    let result = resurrect_at_corpse(
        death_state.state,
        position.x,
        position.y,
        position.z,
        WORLD_MAP_ID,
        corpse,
    )
    .map_err(|_| "you are not close enough to your corpse".to_string())?;
    position.x = result.x;
    position.y = result.y;
    position.z = result.z;
    complete_resurrection(health, mana, death_state);
    Ok("resurrected at corpse".into())
}

fn resurrect_at_spirit_healer(
    position: &mut Position,
    health: &mut Health,
    mana: Option<Mut<Mana>>,
    durability: Option<Mut<EquippedDurability>>,
    death_state: &mut PlayerDeathState,
    level: u8,
    now: u64,
) -> Result<String, String> {
    let graveyard = death_state
        .graveyard
        .as_ref()
        .ok_or_else(|| "spirit healer is unavailable".to_string())?;
    let result = accept_spirit_healer(&SpiritHealerContext {
        state: death_state.state,
        player_level: level,
        player_pos: (position.x, position.y, position.z),
        healer_pos: (graveyard.x, graveyard.y, graveyard.z),
        healer_map: graveyard.map_id,
        now,
    })
    .map_err(|_| "you must be near a spirit healer".to_string())?;
    position.x = result.x;
    position.y = result.y;
    position.z = result.z;
    if let Some(mut durability) = durability {
        crate::durability::apply_spirit_healer_loss(&mut durability);
    }
    complete_resurrection(health, mana, death_state);
    Ok("accepted spirit healer resurrection".into())
}

fn complete_resurrection(
    health: &mut Health,
    mut mana: Option<Mut<Mana>>,
    death_state: &mut PlayerDeathState,
) {
    death_state.state = DeathState::Alive;
    death_state.corpse = None;
    death_state.graveyard = None;
    health.current = (health.max * GHOST_RESURRECT_HEALTH_PCT).max(1.0);
    if let Some(mana) = mana.as_mut() {
        mana.current = mana.max * GHOST_RESURRECT_MANA_PCT;
    }
}

fn send_update(
    conn_entity: Entity,
    position: &Position,
    death_state: PlayerDeathState,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<DeathStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DeathChannel>(DeathStateUpdate {
        snapshot: Some(build_snapshot(position, &death_state)),
        message,
        error,
    });
}

fn send_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<DeathStateUpdate>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DeathChannel>(DeathStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

fn build_snapshot(position: &Position, death_state: &PlayerDeathState) -> DeathSnapshot {
    DeathSnapshot {
        state: map_state_snapshot(death_state.state),
        corpse: death_state.corpse.as_ref().map(snapshot_from_corpse),
        graveyard: death_state.graveyard.as_ref().map(snapshot_from_graveyard),
        can_resurrect_at_corpse: death_state
            .corpse
            .as_ref()
            .is_some_and(|corpse| in_corpse_range(position.x, position.y, position.z, corpse)),
        spirit_healer_available: death_state.graveyard.as_ref().is_some_and(|graveyard| {
            let dx = position.x - graveyard.x;
            let dy = position.y - graveyard.y;
            let dz = position.z - graveyard.z;
            dx * dx + dy * dy + dz * dz
                <= shared::death::SPIRIT_HEALER_RANGE * shared::death::SPIRIT_HEALER_RANGE
        }),
    }
}

fn map_state_snapshot(state: DeathState) -> DeathStateSnapshot {
    match state {
        DeathState::Alive => DeathStateSnapshot::Alive,
        DeathState::Dead => DeathStateSnapshot::Dead,
        DeathState::Ghost => DeathStateSnapshot::Ghost,
        DeathState::Resurrecting => DeathStateSnapshot::Resurrecting,
    }
}

fn snapshot_from_corpse(corpse: &Corpse) -> DeathPositionSnapshot {
    DeathPositionSnapshot {
        map_id: corpse.map_id,
        x: corpse.x,
        y: corpse.y,
        z: corpse.z,
    }
}

fn snapshot_from_graveyard(graveyard: &Graveyard) -> DeathPositionSnapshot {
    DeathPositionSnapshot {
        map_id: graveyard.map_id,
        x: graveyard.x,
        y: graveyard.y,
        z: graveyard.z,
    }
}

fn current_seconds(time: &Time) -> u64 {
    time.elapsed_secs_f64().floor() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_graveyard_registry_uses_spirit_healer_spawns() {
        let world_data = WorldData {
            templates: [(
                5,
                crate::world_data::CreatureTemplate {
                    entry: 5,
                    min_level: 1,
                    max_level: 1,
                    faction: 0,
                    npc_flags: shared::death::NPC_FLAG_SPIRIT_HEALER,
                    speed_walk: 1.0,
                    health_modifier: 1.0,
                    models: vec![],
                },
            )]
            .into_iter()
            .collect(),
            spawns: vec![crate::world_data::CreatureSpawn {
                guid: 44,
                template_entry: 5,
                map: 0,
                zone_id: 12,
                position_x: -9000.0,
                position_y: 100.0,
                position_z: 50.0,
                wander_distance: 0.0,
                movement_type: 0,
            }],
            spawns_by_map: [(0, vec![0])].into_iter().collect(),
            waypoints: default(),
        };

        let registry = build_graveyard_registry(&world_data);

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn corpse_resurrection_clears_death_state() {
        let mut position = Position {
            x: 10.0,
            y: 0.0,
            z: 10.0,
        };
        let mut health = Health {
            current: 0.0,
            max: 100.0,
        };
        let mut death_state = PlayerDeathState {
            state: DeathState::Ghost,
            corpse: Some(Corpse {
                player: 1,
                map_id: 0,
                x: 10.0,
                y: 0.0,
                z: 10.0,
                created_at: 5,
            }),
            graveyard: Some(Graveyard {
                id: 2,
                zone_id: 12,
                faction: 0,
                map_id: 0,
                x: 50.0,
                y: 0.0,
                z: 50.0,
            }),
        };

        let message =
            resurrect_player_at_corpse(&mut position, &mut health, None, &mut death_state)
                .expect("corpse resurrection");

        assert_eq!(message, "resurrected at corpse");
        assert_eq!(death_state.state, DeathState::Alive);
        assert!(death_state.corpse.is_none());
        assert!(health.current > 0.0);
    }

    #[test]
    fn snapshot_flags_spirit_healer_proximity() {
        let position = Position {
            x: 5.0,
            y: 0.0,
            z: 0.0,
        };
        let death_state = PlayerDeathState {
            state: DeathState::Ghost,
            corpse: None,
            graveyard: Some(Graveyard {
                id: 1,
                zone_id: 12,
                faction: 0,
                map_id: 0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
        };

        let snapshot = build_snapshot(&position, &death_state);

        assert_eq!(snapshot.state, DeathStateSnapshot::Ghost);
        assert!(snapshot.spirit_healer_available);
    }

    #[test]
    fn stuck_escape_teleports_alive_player_to_nearest_graveyard() {
        let mut position = Position {
            x: 90.0,
            y: 5.0,
            z: 0.0,
        };
        let mut graveyards = GraveyardRegistry::new();
        graveyards.add(Graveyard {
            id: 1,
            zone_id: 12,
            faction: 0,
            map_id: WORLD_MAP_ID,
            x: 10.0,
            y: 0.0,
            z: 0.0,
        });
        graveyards.add(Graveyard {
            id: 2,
            zone_id: 12,
            faction: 0,
            map_id: WORLD_MAP_ID,
            x: 100.0,
            y: 0.0,
            z: 0.0,
        });

        let message = use_stuck_escape(&mut position, None, &graveyards).expect("stuck escape");

        assert_eq!(message, "stuck escape used");
        assert_eq!(position.x, 100.0);
        assert_eq!(position.y, 0.0);
        assert_eq!(position.z, 0.0);
    }

    #[test]
    fn stuck_escape_rejects_dead_players() {
        let mut position = Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let death_state = PlayerDeathState {
            state: DeathState::Ghost,
            corpse: None,
            graveyard: None,
        };

        let error = use_stuck_escape(&mut position, Some(&death_state), &GraveyardRegistry::new())
            .expect_err("dead players cannot use stuck escape");

        assert_eq!(error, "stuck escape is unavailable while dead");
    }
}
