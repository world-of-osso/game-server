use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Level, Position};
use shared::protocol::{RestAreaKindSnapshot, RestChannel, RestSnapshot, RestStateUpdate};
use shared::xp::{RestedXp, xp_to_level};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;
use crate::world_data::WorldData;

const NPC_FLAG_INNKEEPER: u32 = 0x0001_0000;
const INNKEEPER_REST_RADIUS: f32 = 20.0;
const INNKEEPER_REST_RADIUS_SQ: f32 = INNKEEPER_REST_RADIUS * INNKEEPER_REST_RADIUS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestAreaKind {
    Inn,
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct PlayerRestState {
    rested_xp: RestedXp,
    area_kind: Option<RestAreaKind>,
    last_sent: Option<RestSnapshot>,
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
            (initialize_rest_area_registry, update_player_rest_state).chain(),
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
        &'static Position,
        &'static Level,
        Option<&'static mut PlayerRestState>,
    ),
    Or<(Changed<Position>, Changed<Level>, Without<PlayerRestState>)>,
>;

fn update_player_rest_state(
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut players: RestPlayerQuery,
    registry: Res<RestAreaRegistry>,
    mut commands: Commands,
    mut senders: Query<&mut MessageSender<RestStateUpdate>>,
) {
    if !registry.initialized {
        return;
    }

    for (conn_entity, link) in &connections {
        let Ok((position, level, rest_state)) = players.get_mut(link.0) else {
            continue;
        };

        let Some(snapshot) = (if let Some(rest_state) = rest_state {
            update_existing_rest_state(position, level, rest_state, &registry)
        } else {
            let snapshot = build_initial_rest_snapshot(position, level.0, &registry);
            commands.entity(link.0).insert(PlayerRestState {
                rested_xp: RestedXp::new(xp_to_level(level.0)),
                area_kind: map_rest_area_kind(snapshot.rest_area_kind.as_ref()),
                last_sent: Some(snapshot.clone()),
            });
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

fn update_existing_rest_state(
    position: &Position,
    level: &Level,
    mut state: Mut<PlayerRestState>,
    registry: &RestAreaRegistry,
) -> Option<RestSnapshot> {
    state.rested_xp.update_max(xp_to_level(level.0));
    state.area_kind = detect_rest_area_kind(position, registry);
    state.rested_xp.in_rest_area = state.area_kind.is_some();

    let snapshot = build_rest_snapshot(&state);
    if state.last_sent.as_ref() == Some(&snapshot) {
        return None;
    }
    state.last_sent = Some(snapshot.clone());
    Some(snapshot)
}

fn build_initial_rest_snapshot(
    position: &Position,
    level: u8,
    registry: &RestAreaRegistry,
) -> RestSnapshot {
    let mut rested_xp = RestedXp::new(xp_to_level(level));
    let area_kind = detect_rest_area_kind(position, registry);
    rested_xp.in_rest_area = area_kind.is_some();
    build_rest_snapshot_from_parts(rested_xp, area_kind)
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

fn build_rest_snapshot(state: &PlayerRestState) -> RestSnapshot {
    build_rest_snapshot_from_parts(state.rested_xp, state.area_kind)
}

fn build_rest_snapshot_from_parts(
    rested_xp: RestedXp,
    area_kind: Option<RestAreaKind>,
) -> RestSnapshot {
    RestSnapshot {
        in_rest_area: rested_xp.in_rest_area,
        rest_area_kind: area_kind.map(map_rest_area_kind_snapshot),
        rested_xp: rested_xp.amount,
        rested_xp_max: rested_xp.max,
    }
}

fn map_rest_area_kind_snapshot(kind: RestAreaKind) -> RestAreaKindSnapshot {
    match kind {
        RestAreaKind::Inn => RestAreaKindSnapshot::Inn,
    }
}

fn map_rest_area_kind(kind: Option<&RestAreaKindSnapshot>) -> Option<RestAreaKind> {
    match kind {
        Some(RestAreaKindSnapshot::Inn) => Some(RestAreaKind::Inn),
        Some(RestAreaKindSnapshot::City) | None => None,
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
    use crate::world_data::{CreatureModelInfo, CreatureSpawn, CreatureTemplate};
    use bevy::ecs::change_detection::DetectChanges;
    use std::collections::HashMap;

    fn rest_template(npc_flags: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry: 1,
            min_level: 1,
            max_level: 1,
            faction: 0,
            npc_flags,
            speed_walk: 1.0,
            health_modifier: 1.0,
            models: vec![CreatureModelInfo { display_id: 1 }],
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
            wander_distance: 0.0,
            movement_type: 0,
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
                Level(1),
                PlayerRestState {
                    rested_xp: RestedXp::new(xp_to_level(1)),
                    area_kind: None,
                    last_sent: Some(RestSnapshot {
                        in_rest_area: false,
                        rest_area_kind: None,
                        rested_xp: 0,
                        rested_xp_max: xp_to_level(1) * 3 / 2,
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
            rested_xp: RestedXp::new(xp_to_level(1)),
            area_kind: None,
            last_sent: None,
        };
        state.area_kind = Some(RestAreaKind::Inn);
        state.rested_xp.in_rest_area = true;

        let snapshot = build_rest_snapshot(&state);
        assert!(snapshot.in_rest_area);
        assert_eq!(snapshot.rest_area_kind, Some(RestAreaKindSnapshot::Inn));
        assert_eq!(snapshot.rested_xp_max, xp_to_level(1) * 3 / 2);
    }
}
