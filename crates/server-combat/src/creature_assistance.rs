//! Formation engagement and creature assistance (TC 15e0ed76 CreatureGroups.cpp:224,
//! Creature.cpp:2551-2652, GridNotifiers.h:1419-1444,1561-1585).

use bevy::prelude::*;
use shared::components::{
    Health, Player, Position, ThreatTable, UnitFactionTemplate, UnitFlags, UnitSummonedBy,
};

use crate::combat::DeathState;
use crate::creature_aggro::{CREATURE_FLAG_EXTRA_CIVILIAN, attack_start, current_victim};
use crate::creature_ai::CreatureState;
use crate::faction::{FactionTemplates, Reaction, UnitFaction};
use crate::grid_activity::GridDormant;
use crate::maps::InMap;
use crate::scripted_ai::ReactState;
use crate::zones::CreatureGroupMember;

// World.cpp:801,915: CreatureFamilyAssistanceDelay and CreatureFamilyAssistanceRadius.
const DEFAULT_ASSISTANCE_DELAY_SECS: f64 = 1.5;
const DEFAULT_ASSISTANCE_RADIUS: f32 = 10.0;
// CreatureGroups.h:28-29: bits on the engaging member, not the other member.
const MEMBERS_ASSIST_LEADER: u32 = 1;
const LEADER_ASSISTS_MEMBER: u32 = 2;

#[derive(Resource)]
pub struct AssistanceConfig {
    pub radius: f32,
    pub delay_secs: f64,
}

impl Default for AssistanceConfig {
    fn default() -> Self {
        Self {
            radius: DEFAULT_ASSISTANCE_RADIUS,
            delay_secs: DEFAULT_ASSISTANCE_DELAY_SECS,
        }
    }
}

#[derive(Component)]
pub(crate) struct AssistanceCalled;

#[derive(Component)]
pub(crate) struct AssistanceSnapshot(pub Vec<Entity>);

#[derive(Component)]
struct FormationEngaging;

struct DelayedAssistance {
    caller: Entity,
    target: Entity,
    assistants: Vec<Entity>,
    due: f64,
}

#[derive(Resource, Default)]
pub(crate) struct PendingAssistance(Vec<DelayedAssistance>);

pub(crate) fn pending_callers(world: &World) -> impl Iterator<Item = Entity> + '_ {
    world
        .get_resource::<PendingAssistance>()
        .into_iter()
        .flat_map(|pending| pending.0.iter().map(|event| event.caller))
}

pub(crate) fn committed_assistants(world: &World) -> impl Iterator<Item = Entity> + '_ {
    world
        .get_resource::<PendingAssistance>()
        .into_iter()
        .flat_map(|pending| &pending.0)
        .flat_map(move |event| {
            event.assistants.iter().copied().filter(move |&assistant| {
                can_assist(world, assistant, event.caller, event.target, true)
            })
        })
}

fn alive(world: &World, entity: Entity) -> bool {
    world
        .get::<Health>(entity)
        .is_some_and(|health| health.current > 0.0)
        && world.get::<DeathState>(entity).is_none()
        && world
            .get::<crate::death::PlayerDeathState>(entity)
            .is_none_or(|death| death.state == shared::death::DeathState::Alive)
}

fn faction(world: &World, entity: Entity) -> Option<UnitFaction> {
    Some(UnitFaction {
        template: world.get::<UnitFactionTemplate>(entity)?.0,
        is_player: crate::faction::player_affected(
            world.get::<Player>(entity).is_some(),
            world.get::<UnitFlags>(entity),
        ),
    })
}

fn valid_target(world: &World, assistant: Entity, target: Entity) -> bool {
    let (Some(actor), Some(enemy), Some(templates)) = (
        faction(world, assistant),
        faction(world, target),
        world.get_resource::<FactionTemplates>(),
    ) else {
        return false;
    };
    alive(world, target)
        && world.get::<InMap>(assistant).is_some()
        && world.get::<InMap>(assistant) == world.get::<InMap>(target)
        && world.get::<CreatureState>(target) != Some(&CreatureState::Returning)
        && world
            .get::<UnitFlags>(target)
            .is_none_or(|flags| flags.can_be_attacked_by(actor.is_player))
        && templates.can_attack(actor, enemy)
}

pub(crate) fn can_assist(
    world: &World,
    assistant: Entity,
    caller: Entity,
    target: Entity,
    same_faction: bool,
) -> bool {
    if assistant == caller
        || !alive(world, assistant)
        || world.get::<ThreatTable>(assistant).is_none()
        || world
            .get::<crate::zones::CreatureFlagsExtra>(assistant)
            .is_some_and(|flags| flags.0 & CREATURE_FLAG_EXTRA_CIVILIAN != 0)
        || world.get::<CreatureState>(assistant) != Some(&CreatureState::Idle)
        || world.get::<GridDormant>(assistant).is_some()
        || world.get::<ReactState>(assistant) == Some(&ReactState::Passive)
        || world.get::<UnitSummonedBy>(assistant).is_some()
        || world
            .get::<crate::pet_control::PetCharm>(assistant)
            .is_some()
        || crate::combat_refs::active_opponents(world, assistant)
            .next()
            .is_some()
    {
        return false;
    }
    let blocked = UnitFlags::NON_ATTACKABLE | UnitFlags::IMMUNE_TO_NPC | UnitFlags::NOT_SELECTABLE;
    if world
        .get::<UnitFlags>(assistant)
        .is_some_and(|flags| flags.0 & blocked != 0)
        || !valid_target(world, assistant, target)
    {
        return false;
    }
    let (Some(assistant_faction), Some(caller_faction), Some(target_faction), Some(templates)) = (
        faction(world, assistant),
        faction(world, caller),
        faction(world, target),
        world.get_resource::<FactionTemplates>(),
    ) else {
        return false;
    };
    let friendly = if same_faction {
        assistant_faction.template == caller_faction.template
    } else {
        templates.reaction(assistant_faction.template, caller_faction.template)
            == Reaction::Friendly
    };
    friendly
        && templates.reaction(assistant_faction.template, target_faction.template)
            == Reaction::Hostile
}

fn nearby(world: &World, caller: Entity, radius: f32) -> Vec<Entity> {
    let (Some(position), Some(map), Some(grid)) = (
        world.get::<Position>(caller),
        world.get::<InMap>(caller),
        world.get_resource::<crate::creature_sight::NpcGrid>(),
    ) else {
        return Vec::new();
    };
    grid.within_cells(*map, position, radius)
        .into_iter()
        .filter(|other| {
            world.get::<InMap>(*other) == Some(map)
                && world.get::<Position>(*other).is_some_and(|other| {
                    let dx = position.x - other.x;
                    let dy = position.y - other.y;
                    let dz = position.z - other.z;
                    dx * dx + dy * dy + dz * dz < radius * radius
                })
        })
        .collect()
}

fn sees(world: &World, observer: Entity, target: Entity) -> bool {
    world
        .get_resource::<crate::line_of_sight::LineOfSight>()
        .is_some()
        && crate::line_of_sight::sees_in_world(world, observer, target)
}

/// TC MemberEngagingTarget: the initiating row controls which members assist.
pub(crate) fn engage_formation(world: &mut World, member: Entity, target: Entity) {
    let Some(group) = world.get::<CreatureGroupMember>(member).cloned() else {
        return;
    };
    if world.get::<FormationEngaging>(group.leader).is_some() {
        return;
    }
    let required = if member == group.leader {
        MEMBERS_ASSIST_LEADER
    } else {
        LEADER_ASSISTS_MEMBER
    };
    if group.group_ai & required == 0 {
        return;
    }
    let others: Vec<Entity> = world
        .query::<(Entity, &CreatureGroupMember)>()
        .iter(world)
        .filter(|(other, info)| *other != member && info.leader == group.leader)
        .filter(|(other, _)| {
            let flag = if *other == group.leader {
                LEADER_ASSISTS_MEMBER
            } else {
                MEMBERS_ASSIST_LEADER
            };
            group.group_ai & flag != 0
                && alive(world, *other)
                && valid_target(world, *other, target)
        })
        .map(|(other, _)| other)
        .collect();
    if world.get_entity(group.leader).is_err() {
        return;
    }
    world.entity_mut(group.leader).insert(FormationEngaging);
    for other in others {
        attack_start(world, other, target);
    }
    if let Ok(mut leader) = world.get_entity_mut(group.leader) {
        leader.remove::<FormationEngaging>();
    }
}

/// Read the same immediate helper snapshot used by the native assistance call.
pub(crate) fn assistance_snapshot(world: &World, caller: Entity, target: Entity) -> Vec<Entity> {
    let Some(config) = world.get_resource::<AssistanceConfig>() else {
        return Vec::new();
    };
    let radius = config.radius;
    if radius <= 0.0 {
        return Vec::new();
    }
    nearby(world, caller, radius)
        .into_iter()
        .filter(|&other| {
            can_assist(world, other, caller, target, true) && sees(world, caller, other)
        })
        .collect()
}

/// Unit::Attack calls once per combat; assistants cannot call a second wave.
pub(crate) fn call_assistance(world: &mut World, caller: Entity, target: Entity) {
    if world.get::<shared::components::Npc>(caller).is_none()
        || world.get::<AssistanceCalled>(caller).is_some()
        || world.get::<crate::pet_control::PetCharm>(caller).is_some()
        || crate::faction::player_affected(false, world.get::<UnitFlags>(caller))
    {
        return;
    }
    world.entity_mut(caller).insert(AssistanceCalled);
    world.init_resource::<AssistanceConfig>();
    world.init_resource::<PendingAssistance>();
    let config = world.resource::<AssistanceConfig>();
    let radius = config.radius;
    let delay = config.delay_secs;
    if radius <= 0.0 {
        return;
    }
    let assistants = assistance_snapshot(world, caller, target);
    if !assistants.is_empty() {
        info!(target: "game_server::creature_assistance", caller = caller.to_bits(), victim = target.to_bits(), count = assistants.len(), "planned assistance snapshot");
    }
    world
        .entity_mut(caller)
        .insert(AssistanceSnapshot(assistants.clone()));
    if assistants.is_empty() {
        return;
    }
    for &assistant in &assistants {
        let distance = world
            .get::<Position>(caller)
            .zip(world.get::<Position>(assistant))
            .map(|(a, b)| Vec3::new(a.x, a.y, a.z).distance(Vec3::new(b.x, b.y, b.z)));
        info!(target: "game_server::creature_assistance", caller = caller.to_bits(), helper = assistant.to_bits(), victim = target.to_bits(), ?distance, delay_secs = delay, "assistance scheduled");
    }
    let due = world.resource::<Time>().elapsed_secs_f64() + delay;
    world
        .resource_mut::<PendingAssistance>()
        .0
        .push(DelayedAssistance {
            caller,
            target,
            assistants,
            due,
        });
}

/// TC CallForHelp checks friendly factions and the assistant's LOS to the enemy.
pub fn call_for_help(world: &mut World, caller: Entity, radius: f32) {
    if radius <= 0.0
        || !alive(world, caller)
        || world.get::<crate::pet_control::PetCharm>(caller).is_some()
        || crate::combat_refs::active_opponents(world, caller)
            .next()
            .is_none()
    {
        return;
    }
    let target = current_victim(world, caller)
        .or_else(|| {
            world.get::<ThreatTable>(caller).and_then(|table| {
                table
                    .entries()
                    .iter()
                    .find_map(|entry| Entity::try_from_bits(entry.entity))
            })
        })
        .or_else(|| crate::combat_refs::active_opponents(world, caller).next());
    let Some(target) = target else {
        return;
    };
    let assistants: Vec<_> = nearby(world, caller, radius)
        .into_iter()
        .filter(|&other| {
            can_assist(world, other, caller, target, false) && sees(world, other, target)
        })
        .collect();
    for assistant in assistants {
        attack_start(world, assistant, target);
    }
}

pub(crate) fn dispatch_assistance(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let pending = std::mem::take(&mut world.resource_mut::<PendingAssistance>().0);
    let (due, later): (Vec<_>, Vec<_>) = pending.into_iter().partition(|event| event.due <= now);
    world.resource_mut::<PendingAssistance>().0 = later;
    for event in due {
        for assistant in event.assistants {
            if can_assist(world, assistant, event.caller, event.target, true) {
                info!(target: "game_server::creature_assistance", caller = event.caller.to_bits(), helper = assistant.to_bits(), victim = event.target.to_bits(), due = event.due, now, "assistance dispatched");
                world.entity_mut(assistant).insert(AssistanceCalled);
                attack_start(world, assistant, event.target);
            }
        }
    }
}
