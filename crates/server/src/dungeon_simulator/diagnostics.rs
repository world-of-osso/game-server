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
        "npc_entry": world.get::<shared::components::Npc>(actor).map(|npc| npc.template_id),
        "unit_level": world.get::<shared::components::UnitLevel>(actor).map(|level| level.0),
        "hunter_pet_owner": world.get::<crate::hunter_pet::HunterPet>(actor).map(|pet| pet.owner.to_bits()),
        "summoned_by": world.get::<shared::components::UnitSummonedBy>(actor).map(|summoned| summoned.0),
        "follower_spec": world.get::<crate::followers::Follower>(actor).map(|follower| follower.profile.spec_id),
        "level": world.get::<shared::components::Level>(actor).map(|level| level.0),
        "map": format!("{:?}", world.get::<crate::maps::InMap>(actor)),
        "faction": world.get::<shared::components::UnitFactionTemplate>(actor).map(|faction| faction.0),
        "powers": format!("{:?}", world.get::<crate::spell_cast::powers::PowerPools>(actor)),
        "mana": world.get::<shared::components::Mana>(actor).map(|mana| [mana.current, mana.max]),
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

/// Diagnostic identities persist after native cleanup; never affect combat membership.
#[derive(Default)]
pub(super) struct PetRecorder {
    pets: std::collections::BTreeMap<Entity, PetRecord>,
}

struct PetRecord {
    owner: Entity,
    opponents: std::collections::BTreeSet<Entity>,
}

impl PetRecorder {
    pub(super) fn units(&self) -> impl Iterator<Item = Entity> + '_ {
        self.pets.keys().copied()
    }

    pub(super) fn sample(
        &mut self,
        world: &World,
        members: &[Entity],
        actors: &std::collections::BTreeMap<u32, Entity>,
    ) -> Vec<serde_json::Value> {
        for &owner in members {
            if let Some(pet) = crate::hunter_pet::summoned_pet(world, owner) {
                self.pets.entry(pet).or_insert_with(|| PetRecord {
                    owner,
                    opponents: Default::default(),
                });
            }
        }
        self.pets
            .iter_mut()
            .map(|(&pet, record)| {
                record
                    .opponents
                    .extend(crate::combat_refs::active_opponents(world, pet));
                if let Some(target) = world
                    .get::<crate::networking::CombatTarget>(pet)
                    .and_then(|target| target.0)
                {
                    record.opponents.insert(target);
                }
                // Include zero-threat native acquisition, not just positive damage threat.
                record
                    .opponents
                    .extend(actors.values().copied().filter(|actor| {
                        world
                            .get::<shared::components::ThreatTable>(*actor)
                            .is_some_and(|table| {
                                table
                                    .entries()
                                    .iter()
                                    .any(|entry| entry.entity == pet.to_bits())
                            })
                    }));
                let opponents: Vec<_> = record
                    .opponents
                    .iter()
                    .copied()
                    .map(|actor| {
                        let guid = world
                            .get::<crate::zones::CreatureOrigin>(actor)
                            .map(|source| source.guid);
                        inspect_unit(world, guid, actor)
                    })
                    .collect();
                serde_json::json!({"owner":record.owner.to_bits(),
                "pet":inspect_unit(world,None,pet),"opponents":opponents})
            })
            .collect()
    }
}
