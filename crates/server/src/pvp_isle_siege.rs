//! Isle gates: TDB1210 type33 HealthRec28/32 and destroyed-event metadata.
use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GateTemplate {
    pub max_health: u32,
    pub damaged_health: u32,
    pub destroyed_event: u32,
}

#[derive(Resource, Default, Debug)]
pub(crate) struct Catalog(pub HashMap<u32, GateTemplate>);

impl Catalog {
    pub fn load(source: &Connection) -> Result<Self, String> {
        let gates = shared::battleground::isle_gate_entries()
            .into_iter()
            .map(|entry| load_gate_template(source, entry).map(|template| (entry, template)))
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Self(gates))
    }
}

fn load_gate_template(source: &Connection, entry: u32) -> Result<GateTemplate, String> {
    let (intact, damaged, event): (u32, u32, u32) = source
        .query_row(
            "SELECT h.IntactNumHits,h.DamagedNumHits,t.Data14 FROM content_gameobject_template t
         JOIN tdb_destructible_hitpoint h ON h.Id=t.Data2 WHERE t.entry=?1 AND t.type=33",
            [entry],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| format!("Isle gate {entry} source HealthRec/destroyed event: {error}"))?;
    let max_health = intact
        .checked_add(damaged)
        .filter(|health| *health > 0)
        .ok_or_else(|| format!("Isle gate {entry} has invalid source health"))?;
    if event == 0 {
        return Err(format!(
            "Isle gate {entry} lacks its source destroyed event"
        ));
    }
    Ok(GateTemplate {
        max_health,
        damaged_health: damaged,
        destroyed_event: event,
    })
}

#[derive(Component)]
struct GateHealth {
    current: u32,
    damaged: bool,
    template: GateTemplate,
}

pub(super) fn initialize_gates(world: &mut World, map: crate::maps::InMap) {
    if map.map_id != 628 {
        return;
    }
    let entries = shared::battleground::isle_gate_entries();
    let mut objects = world.query_filtered::<(
        Entity,
        &shared::protocol::GameObjectInfo,
        &crate::maps::InMap,
    ), Without<GateHealth>>();
    let gates: Vec<_> = objects
        .iter(world)
        .filter(|(_, info, copy)| **copy == map && entries.contains(&info.entry))
        .map(|(entity, info, _)| (entity, info.entry))
        .collect();
    for (entity, entry) in gates {
        let template = *world
            .resource::<Catalog>()
            .0
            .get(&entry)
            .expect("validated source Isle gate");
        world.entity_mut(entity).insert(GateHealth {
            current: template.max_health,
            damaged: false,
            template,
        });
    }
}

/// SPELL_EFFECT_GAMEOBJECT_DAMAGE87; the target selector supplies physical source GOs.
pub(crate) fn effect_damage(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    effect: &crate::spell_info::SpellEffect,
    target: Entity,
) {
    let Some(map) = world.get::<crate::maps::InMap>(target).copied() else {
        return;
    };
    if world.get::<crate::maps::InMap>(ctx.caster) != Some(&map) {
        return;
    }
    if world.get::<GateHealth>(target).is_none() {
        return;
    }
    if !has_active_match(world, map) {
        debug!(gate=?target, "Isle gate damage refused outside active match");
        return;
    }
    let amount = effect.base_points.max(0.0) as u32;
    let mut health = world
        .get_mut::<GateHealth>(target)
        .expect("source gate health");
    let destroyed_event = subtract_gate_health(&mut health, amount);
    debug!(gate=?target, amount, health=health.current, damaged=health.damaged, "Isle gate damage evaluated");
    if let Some(event) = destroyed_event {
        apply_gate_destruction(world, target, map, event);
    }
}

fn has_active_match(world: &World, map: crate::maps::InMap) -> bool {
    world
        .resource::<super::PvpServerState>()
        .battleground_matches
        .values()
        .any(|matched| {
            matched.map == Some(map)
                && matched.instance.bg_id == 30
                && matched.instance.phase == shared::battleground::BgPhase::InProgress
        })
}

fn subtract_gate_health(health: &mut GateHealth, amount: u32) -> Option<u32> {
    if health.current == 0 || amount == 0 {
        return None;
    }
    health.current = health.current.saturating_sub(amount);
    health.damaged |= health.current <= health.template.damaged_health;
    (health.current == 0).then_some(health.template.destroyed_event)
}

fn apply_gate_destruction(world: &mut World, target: Entity, map: crate::maps::InMap, event: u32) {
    let entry = world
        .get::<shared::protocol::GameObjectInfo>(target)
        .expect("source gate object")
        .entry;
    let result = world.resource_scope(|_, mut state: Mut<super::PvpServerState>| {
        let (id, matched) = state
            .battleground_matches
            .iter_mut()
            .find(|(_, matched)| matched.map == Some(map))
            .ok_or("no active Isle map copy")?;
        let runtime = matched.alterac.as_mut().ok_or("no Isle banner runtime")?;
        let super::pvp_alterac::BannerBattle::Isle(battle) = &mut runtime.battle else {
            return Err("gate is not in an Isle match");
        };
        let team = battle.gate_destroyed(entry, &matched.instance)?;
        Ok((*id, team))
    });
    match result {
        Ok((match_id, team)) => {
            open_gate_and_keep(world, target, map, team);
            debug!(entry, event, "Isle source gate destroyed");
            super::pvp_objective::send_match_snapshot(world, match_id);
        }
        Err(reason) => error!(entry, event, reason, "Isle gate destruction refused"),
    }
}

fn open_gate_and_keep(
    world: &mut World,
    target: Entity,
    map: crate::maps::InMap,
    team: shared::battleground::BgTeam,
) {
    let doors = shared::battleground::isle_keep_door_entries(team);
    let mut objects = world.query::<(
        Entity,
        &shared::protocol::GameObjectInfo,
        &crate::maps::InMap,
    )>();
    let opened: Vec<_> = objects
        .iter(world)
        .filter(|(entity, info, copy)| {
            *entity == target || (**copy == map && doors.contains(&info.entry))
        })
        .map(|(entity, _, _)| entity)
        .collect();
    for entity in opened {
        world
            .entity_mut(entity)
            .insert(shared::protocol::GameObjectCollision(false));
        if entity != target {
            world
                .entity_mut(entity)
                .insert(crate::game_objects::GameObjectState(
                    crate::game_objects::GO_STATE_ACTIVE,
                ));
        }
        if let Some(dirty) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
            dirty.mark(entity);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn source_catalog() -> Connection {
        let source = Connection::open_in_memory().unwrap();
        source.execute_batch("CREATE TABLE content_gameobject_template (entry INTEGER PRIMARY KEY, type INTEGER, Data2 INTEGER, Data14 INTEGER);
            CREATE TABLE tdb_destructible_hitpoint (Id INTEGER PRIMARY KEY, IntactNumHits INTEGER, DamagedNumHits INTEGER);
            INSERT INTO tdb_destructible_hitpoint VALUES (28,1,600000),(32,1,600000);
            INSERT INTO content_gameobject_template VALUES
            (195494,33,28,22079),(195495,33,28,22083),(195496,33,28,22081),
            (195698,33,32,22078),(195699,33,32,22082),(195700,33,32,22080);").unwrap();
        source
    }

    #[test]
    fn isle_gate_catalog_preserves_real_hitpoints_and_destroyed_events() {
        let source = source_catalog();
        let catalog = Catalog::load(&source).unwrap();
        assert_eq!(catalog.0.len(), 6);
        for (entry, event) in [
            (195494, 22079),
            (195495, 22083),
            (195496, 22081),
            (195698, 22078),
            (195699, 22082),
            (195700, 22080),
        ] {
            assert_eq!(
                catalog.0[&entry],
                GateTemplate {
                    max_health: 600001,
                    damaged_health: 600000,
                    destroyed_event: event
                }
            );
        }
    }

    #[test]
    fn gameobject_area_selects_physical_objects_in_the_same_copy_not_nearby_units() {
        use shared::components::Position;
        use shared::protocol::GameObjectInfo;
        let mut world = World::new();
        let map = crate::maps::InMap {
            map_id: 628,
            instance_id: 17,
        };
        let origin = Position {
            x: 100.0,
            y: 20.0,
            z: 30.0,
        };
        let caster = world.spawn((origin, map)).id();
        let info = GameObjectInfo {
            entry: 195494,
            go_type: 33,
            display_id: 9048,
            name: "Horde Gate".into(),
            scale: 1.0,
        };
        let near = world
            .spawn((Position { x: 104.0, ..origin }, map, info.clone()))
            .id();
        world.spawn((Position { x: 106.0, ..origin }, map, info.clone()));
        world.spawn((
            origin,
            crate::maps::InMap {
                instance_id: 18,
                ..map
            },
            info,
        ));
        world.spawn((
            origin,
            map,
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
        ));
        let spell = crate::spell_info::SpellInfo {
            id: 66672,
            ..default()
        };
        let effect = crate::spell_info::SpellEffect {
            effect: 87,
            base_points: 6000.0,
            implicit_targets: [52, 0],
            radius: [
                Some(crate::spell_info::SpellRadius {
                    radius: 5.0,
                    ..default()
                }),
                None,
            ],
            ..default()
        };
        let targets = crate::spell_cast::targets::select_targets(
            &mut world, caster, None, &spell, &effect, 0, false,
        )
        .unwrap();
        assert_eq!(targets, vec![near]);
    }

    #[test]
    fn source_bomb_damage_crosses_damaged_threshold_and_destroys_once_at_zero() {
        let template = GateTemplate {
            max_health: 600001,
            damaged_health: 600000,
            destroyed_event: 22079,
        };
        let mut gate = GateHealth {
            current: template.max_health,
            damaged: false,
            template,
        };
        assert_eq!(subtract_gate_health(&mut gate, 0), None);
        assert!(!gate.damaged);
        for _ in 0..100 {
            assert_eq!(subtract_gate_health(&mut gate, 6000), None);
        }
        assert_eq!(gate.current, 1);
        assert!(gate.damaged);
        assert_eq!(subtract_gate_health(&mut gate, 6000), Some(22079));
        assert_eq!(gate.current, 0);
        assert_eq!(subtract_gate_health(&mut gate, 6000), None);
    }

    #[test]
    fn gate_damage_cannot_consume_source_hitpoints_before_match_start() {
        let mut world = World::new();
        world.insert_resource(super::super::PvpServerState::default());
        let map = crate::maps::InMap {
            map_id: 628,
            instance_id: 17,
        };
        let caster = world.spawn(map).id();
        let template = GateTemplate {
            max_health: 600001,
            damaged_health: 600000,
            destroyed_event: 22079,
        };
        let gate = world
            .spawn((
                map,
                GateHealth {
                    current: template.max_health,
                    damaged: false,
                    template,
                },
            ))
            .id();
        let spell = crate::spell_info::SpellInfo {
            id: 66672,
            ..default()
        };
        let effect = crate::spell_info::SpellEffect {
            effect: 87,
            base_points: 6000.0,
            ..default()
        };
        let ctx = crate::spell_cast::effects::EffectCtx {
            caster,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        effect_damage(&mut world, &ctx, &effect, gate);
        assert_eq!(world.get::<GateHealth>(gate).unwrap().current, 600001);
        assert!(!world.get::<GateHealth>(gate).unwrap().damaged);
    }

    #[test]
    fn isle_gate_catalog_refuses_missing_source_health_instead_of_inventing_it() {
        let source = source_catalog();
        source
            .execute("DELETE FROM tdb_destructible_hitpoint WHERE Id=28", [])
            .unwrap();
        assert!(Catalog::load(&source).unwrap_err().contains("195494"));
    }
}
