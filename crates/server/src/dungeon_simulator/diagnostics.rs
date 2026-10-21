//! Read-only source-state snapshots and explicit profiling checkpoints.
use bevy::prelude::*;
use shared::components::{Health, Position};

pub(super) fn inspect_unit(world: &World, guid: Option<u32>, actor: Entity) -> serde_json::Value {
    let health = world.get::<Health>(actor);
    let position = world.get::<Position>(actor);
    let threat: Vec<_> = world
        .get::<shared::components::ThreatTable>(actor)
        .into_iter()
        .flat_map(|table| table.entries())
        .map(|entry| serde_json::json!({"entity": entry.entity, "threat": entry.threat}))
        .collect();
    let auras: Vec<_> = world.get::<crate::auras::ActiveAuras>(actor).into_iter().flat_map(|auras| auras.iter())
        .map(|aura| serde_json::json!({"spell": aura.spell_id, "expires_at": aura.expires_at, "caster": aura.caster.to_bits()})).collect();
    serde_json::json!({"guid": guid, "entity": actor.to_bits(), "exists": world.get_entity(actor).is_ok(),
        "name": world.get::<shared::components::Npc>(actor).map(|npc| &npc.name),
        "health": health.map(|health| [health.current, health.max]),
        "position": position.map(|position| [position.x, position.y, position.z]),
        "state": format!("{:?}", world.get::<crate::creature_ai::CreatureState>(actor)),
        "target": world.get::<crate::networking::CombatTarget>(actor).and_then(|target| target.0).map(Entity::to_bits),
        "auto_attack": format!("{:?}", world.get::<crate::combat::AutoAttack>(actor)),
        "in_combat": world.get::<shared::components::CombatStatus>(actor).map(|status| status.0),
        "cast": format!("{:?}", world.get::<shared::casting::CastState>(actor)),
        "follow": format!("{:?}", world.get::<crate::creature_ai::FollowOwner>(actor)),
        "reach": world.get::<crate::followers::FollowerReach>(actor).map(|reach| reach.0),
        "flags": world.get::<shared::components::UnitFlags>(actor).map(|flags| flags.0),
        "combat_refs": format!("{:?}", world.get::<crate::combat_refs::CombatRefs>(actor)),
        "threat": threat, "auras": auras})
}

pub(super) fn inspect_aggro_pair(
    world: &mut World,
    watcher: Entity,
    target: Entity,
) -> Result<serde_json::Value, String> {
    use shared::components::UnitLevel;
    use shared::level_scaling::{LevelScaling, level_for_viewer};
    let native = |unit| {
        world
            .get::<UnitLevel>(unit)
            .copied()
            .ok_or_else(|| format!("aggro diagnostic lacks level for {unit:?}"))
    };
    let watcher_level = native(watcher)?;
    let target_level = native(target)?;
    let seen_watcher = level_for_viewer(
        watcher_level,
        world.get::<LevelScaling>(watcher),
        target_level.0,
    );
    let seen_target = level_for_viewer(
        target_level,
        world.get::<LevelScaling>(target),
        watcher_level.0,
    );
    let mut state =
        bevy::ecs::system::SystemState::<crate::creature_aggro::AggroDistance>::new(world);
    let geometry = state
        .get(world)
        .map_err(|error| format!("aggro diagnostic parameters: {error:?}"))?;
    Ok(
        serde_json::json!({"watcher": watcher.to_bits(), "target": target.to_bits(),
        "watcher_native_level": watcher_level.0, "target_native_level": target_level.0,
        "watcher_seen_level": seen_watcher, "target_seen_level": seen_target,
        "watcher_reach": geometry.combat_reach(watcher), "target_reach": geometry.combat_reach(target),
        "npc_acquisition_radius": geometry.center_radius(watcher, target, seen_watcher, seen_target),
        "projection_radius": geometry.center_radius(watcher, target, seen_watcher, target_level.0)}),
    )
}

pub(super) fn read_process_memory() -> serde_json::Value {
    let status =
        std::fs::read_to_string("/proc/self/status").expect("read simulator process memory status");
    let memory: std::collections::BTreeMap<_, _> = status
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            ["VmRSS", "VmHWM", "VmSize", "VmSwap"]
                .contains(&key)
                .then(|| (key, value.trim()))
        })
        .collect();
    serde_json::to_value(memory).unwrap()
}
