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
