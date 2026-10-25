//! Explicit admin harness placement, never ordinary follower movement or transport.
use super::*;
use shared::components::{CombatStatus, Npc, Player, Position};
use std::collections::BTreeSet;

pub(crate) fn stage_follower_party(
    world: &mut World,
    name: &str,
    destination: WorldLocation,
) -> Result<usize, String> {
    let owner = world
        .query::<(Entity, &Player, &ClientId)>()
        .iter(world)
        .find(|(_, player, _)| player.name == name)
        .map(|(entity, _, _)| entity)
        .ok_or("staging owner must be online")?;
    let (copy, members) = require_idle_owned_party(world, owner, destination)?;
    if !teleport_to(world, owner, destination, TeleportOptions::default()) {
        return Err("ordinary owner teleport refused".into());
    }
    let position = *world.get::<Position>(owner).expect("teleported owner");
    for &member in &members {
        if let Some(mut navigation) = world.get_resource_mut::<crate::navigation::Navigation>() {
            navigation.cancel_route(member);
        }
        world.entity_mut(member).insert(position);
        crate::interest::mark_interest_position_changed(
            world.get_resource::<crate::interest::InterestDirtyQueue>(),
            member,
        );
    }
    info!(
        "HARNESS follower-party staging: owner {name}, copy {copy:?}, {} existing followers; not gameplay traversal",
        members.len()
    );
    Ok(members.len())
}

fn require_idle_owned_party(
    world: &mut World,
    owner: Entity,
    destination: WorldLocation,
) -> Result<(InMap, Vec<Entity>), String> {
    if !destination.position.is_finite() {
        return Err("staging coordinates must be finite".into());
    }
    let copy = *world.get::<InMap>(owner).ok_or("owner is transferring")?;
    if copy.map_id != destination.map_id || copy.instance_id == 0 {
        return Err("staging destination must remain in the owner's admitted copy".into());
    }
    if world.get::<MapTransfer>(owner).is_some() {
        return Err("owner is transferring".into());
    }
    let run = world
        .get::<FollowerRun>(owner)
        .ok_or("owner has no genuine follower run")?;
    if run.copy != Some(copy) || run.members.len() != 4 || run.profiles.len() != 4 {
        return Err("owner's genuine follower party is incomplete".into());
    }
    let members = run.members.clone();
    require_genuine_members(world, owner, copy, run)?;
    require_selected_copy(world, owner, copy)?;
    require_idle_copy(world, copy)?;
    Ok((copy, members))
}

fn require_selected_copy(world: &World, owner: Entity, copy: InMap) -> Result<(), String> {
    let entrant = crate::maps::transfer::entrant_of(world, owner);
    let now = crate::maps::difficulty::now(world);
    if world
        .resource::<crate::maps::InstanceManager>()
        .find_instance(
            world.resource::<crate::maps::MapStore>(),
            copy.map_id,
            entrant,
            now.unix,
        )
        != Some(copy)
    {
        return Err("owner's admitted follower copy is no longer selected".into());
    }
    Ok(())
}

fn require_idle_copy(world: &mut World, copy: InMap) -> Result<(), String> {
    let encounter_active = world
        .get_resource::<crate::scripted_ai::instance_script::InstanceScripts>()
        .and_then(|scripts| scripts.0.get(&copy))
        .is_some_and(|script| {
            script.bosses.iter().any(|boss| {
                boss.state == crate::scripted_ai::instance_script::EncounterState::InProgress
            })
        });
    let combat_active = world
        .query::<(&InMap, &CombatStatus)>()
        .iter(world)
        .any(|(map, combat)| *map == copy && combat.0);
    if encounter_active || combat_active {
        return Err("staging refused during combat or an encounter in progress".into());
    }
    Ok(())
}

fn require_genuine_members(
    world: &World,
    owner: Entity,
    copy: InMap,
    run: &FollowerRun,
) -> Result<(), String> {
    for (&unit, profile) in run.members.iter().zip(&run.profiles) {
        if !is_genuine_follower(world, unit, owner, copy, profile.entry) {
            return Err("party member is not the owner's genuine follower".into());
        }
    }
    require_owned_roster(world, owner, &run.members)
}

fn is_genuine_follower(
    world: &World,
    unit: Entity,
    owner: Entity,
    copy: InMap,
    entry: u32,
) -> bool {
    let owned_profile = world
        .get::<Follower>(unit)
        .is_some_and(|follower| follower.owner == owner && follower.profile.entry == entry);
    let owned_summon = world
        .get::<crate::zones::TempSummon>(unit)
        .is_some_and(|summon| summon.summoner == owner);
    let native_npc = world
        .get::<Npc>(unit)
        .is_some_and(|npc| npc.template_id == entry);
    owned_profile
        && owned_summon
        && native_npc
        && world.get::<InMap>(unit) == Some(&copy)
        && world.get::<Position>(unit).is_some()
}

fn require_owned_roster(world: &World, owner: Entity, members: &[Entity]) -> Result<(), String> {
    let id = world.get::<ClientId>(owner).expect("online owner").0;
    let expected: BTreeSet<_> = std::iter::once(id)
        .chain(members.iter().copied().map(Follower::member_id))
        .collect();
    let actual: BTreeSet<_> = world
        .resource::<GroupRegistry>()
        .group_members(id)
        .into_iter()
        .collect();
    if actual != expected || expected.len() != 5 {
        return Err("party roster is not the owner's genuine follower party".into());
    }
    Ok(())
}
