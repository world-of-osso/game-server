//! Explicit transport-free simulator seam; real initializer and kit controller.
use super::*;
use bevy::ecs::system::RunSystemOnce;
use shared::components::{Level, Player, Position, UnitFactionTemplate};

#[derive(Component)]
struct LeaderProfile(Profile);

pub(crate) struct Party {
    pub owner: Entity,
    pub tank: Entity,
    pub members: Vec<Entity>,
}

pub(crate) fn spawn_party(
    world: &mut World,
    copy: crate::maps::InMap,
    at: Position,
) -> Result<Party, String> {
    let profiles = world.resource::<FollowerCatalog>().profiles.clone();
    let leader = profiles
        .iter()
        .find(|profile| profile.entry == 209065)
        .cloned()
        .ok_or("Austin source profile absent")?;
    let owner = world
        .spawn((
            Player {
                name: leader.name.clone(),
                race: 1,
                class: leader.class_id,
                appearance: default(),
            },
            Level(32),
            shared::components::UnitLevel(32),
            UnitFactionTemplate(1),
            at,
            copy,
            crate::persistence::ClientId(1),
            crate::maps::DungeonDifficulty(1),
            crate::networking::CombatTarget::default(),
            crate::networking::Moving(false),
            shared::components::UnitPowers::default(),
        ))
        .id();
    // Run native Player initialization before the profile installs PowerPools;
    // otherwise its Without<PowerPools> query never installs health regeneration.
    world
        .run_system_once(crate::spell_cast::powers::init_player_powers)
        .map_err(|error| {
            format!("initialize simulator leader through native player powers: {error}")
        })?;
    spawning::initialize(world, owner, owner, leader.clone())?;
    world
        .entity_mut(owner)
        .remove::<(Follower, shared::components::UnitSummonedBy)>()
        .insert(LeaderProfile(leader));
    let selected: Vec<_> = [209057, 209072, 209053, 209059]
        .into_iter()
        .map(|entry| {
            profiles
                .iter()
                .find(|profile| profile.entry == entry)
                .cloned()
                .ok_or_else(|| format!("source follower profile {entry} absent"))
        })
        .collect::<Result<_, _>>()?;
    let members = spawning::spawn_members(world, owner, &selected)?;
    party::join_members(world, owner, shared::group::GroupRole::Dps, &members)?;
    world
        .entity_mut(owner)
        .insert(stockade_diagnostic::DiagnosticParty {
            copy,
            members: members.clone(),
            player_role: shared::group::GroupRole::Dps,
        });
    let tank = members
        .iter()
        .copied()
        .find(|member| {
            world.get::<Follower>(*member).unwrap().profile.role == shared::group::GroupRole::Tank
        })
        .ok_or("simulator party lacks tank")?;
    Ok(Party {
        owner,
        tank,
        members,
    })
}

pub(crate) fn update_leader(world: &mut World) {
    let leaders: Vec<_> = world
        .query::<(Entity, &LeaderProfile)>()
        .iter(world)
        .map(|(entity, profile)| (entity, profile.0.clone()))
        .collect();
    for (owner, profile) in leaders {
        if crate::spell_cast::targets::is_alive(world, owner) {
            ai::update_follower(world, owner, &Follower { owner, profile });
        }
    }
}

pub(crate) fn pull(
    world: &mut World,
    tank: Entity,
    guids: &std::collections::BTreeSet<u32>,
) -> Result<(bool, String), String> {
    pull_history::sample_all(world);
    let decision = pull_admission::read_pull_budget(world, tank, guids, 1)?;
    let allowed = matches!(decision, pull_budget::Decision::Allow { .. });
    Ok((allowed, format!("{decision:?}")))
}

pub(crate) fn inspect_tank_movement(
    world: &mut World,
    tank: Entity,
) -> Result<Option<bool>, String> {
    let target = world
        .get::<crate::networking::CombatTarget>(tank)
        .and_then(|target| target.0)
        .ok_or("tank has no selected victim")?;
    let follower = world.get::<Follower>(tank).unwrap().clone();
    tank_planner::read_movement_admission(world, tank, target, &follower, None)
}

pub(crate) fn opening_spell(world: &World, tank: Entity) -> (u32, f32) {
    let spell_id = world.get::<Follower>(tank).unwrap().profile.attack_spell;
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    (
        spell_id,
        crate::spell_cast::targets::spell_range(spell, true).1,
    )
}

pub(crate) fn cast_opening(
    world: &mut World,
    tank: Entity,
    target: Entity,
) -> Result<(), crate::spell_cast::CastFailure> {
    let (spell, _) = opening_spell(world, tank);
    crate::spell_cast::creature_cast(world, tank, spell, Some(target), false)
}
