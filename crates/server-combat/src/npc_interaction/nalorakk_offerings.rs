//! HOARDMONGER_UNLOCK_LOG_CORRELATED: pickup and delivery are inferred from logs.
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::{Npc, Position, UnitFlags};

const MAP: u32 = 2825;
const AVATAR: u32 = 241805;
const HOARDMONGER: u32 = 241812;
const CARRY: u32 = 1239428;
const FEAST: u32 = 1249737;
// No source specifies the interaction radius; this is an inferred five yards.
pub(crate) const DELIVERY_RADIUS: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Offering {
    Berries,
    Fish,
    Apples,
}

#[derive(Component)]
pub(crate) struct PendingOffering {
    spell: u32,
    source: Entity,
    kind: Offering,
}
#[derive(Component)]
pub(crate) struct CarryingOffering {
    kind: Offering,
}
#[derive(Component)]
struct AvatarPrepared;
#[derive(Component)]
pub(crate) struct AvatarFed;

fn offering(entry: u32, spell: u32) -> Option<Offering> {
    match (entry, spell) {
        (244941 | 245750, 1239001) => Some(Offering::Berries),
        (245638, 1239040) | (245698, 1239227) => Some(Offering::Fish),
        (245716, 1239300) => Some(Offering::Apples),
        _ => None,
    }
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn is_offering_source(entry: u32) -> bool {
    [1239001, 1239040, 1239227, 1239300]
        .into_iter()
        .any(|spell| offering(entry, spell).is_some())
}

pub(super) fn begin_pickup(
    world: &mut World,
    player: Entity,
    source: Entity,
    entry: u32,
    spell: u32,
) -> bool {
    let Some(kind) = offering(entry, spell) else {
        return false;
    };
    if world
        .get::<InMap>(player)
        .is_none_or(|map| map.map_id != MAP)
    {
        return false;
    }
    if world.get::<CarryingOffering>(player).is_some()
        || world.get::<PendingOffering>(player).is_some()
    {
        return false;
    }
    world.entity_mut(player).insert(PendingOffering {
        spell,
        source,
        kind,
    });
    true
}

pub(super) fn cancel_pickup(world: &mut World, player: Entity) {
    world.entity_mut(player).remove::<PendingOffering>();
}

pub(crate) fn finish_pickup(world: &mut World, player: Entity, spell: u32, target: Option<Entity>) {
    let Some(pending) = world.get::<PendingOffering>(player) else {
        return;
    };
    if pending.spell != spell || Some(pending.source) != target {
        return;
    }
    let kind = pending.kind;
    world.entity_mut(player).remove::<PendingOffering>();
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster: player,
            target: player,
            spell_id: CARRY,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    if crate::auras::has_aura(world, player, CARRY) {
        world.entity_mut(player).insert(CarryingOffering { kind });
    }
}

pub(crate) fn update_offerings(world: &mut World) {
    clear_interrupted_pickups(world);
    prepare_avatars(world);
    let carriers: Vec<_> = world
        .query::<(Entity, &CarryingOffering, &Position, &InMap)>()
        .iter(world)
        .filter(|(_, _, _, map)| map.map_id == MAP)
        .map(|(entity, carrying, pos, map)| (entity, carrying.kind, *pos, *map))
        .collect();
    for (player, _kind, pos, map) in carriers {
        if !crate::auras::has_aura(world, player, CARRY) {
            world.entity_mut(player).remove::<CarryingOffering>();
            continue;
        }
        let avatar = world
            .query::<(Entity, &Npc, &Position, &InMap, Option<&AvatarFed>)>()
            .iter(world)
            .filter(|(_, npc, _, other, fed)| {
                npc.template_id == AVATAR && **other == map && fed.is_none()
            })
            .filter_map(|(entity, _, at, _, _)| {
                let distance = Vec3::new(pos.x, pos.y, pos.z).distance(Vec3::new(at.x, at.y, at.z));
                (distance <= DELIVERY_RADIUS).then_some((entity, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(entity, _)| entity);
        if let Some(avatar) = avatar {
            deliver(world, player, avatar, map);
        }
    }
}

fn clear_interrupted_pickups(world: &mut World) {
    let stopped: Vec<_> = world
        .query_filtered::<Entity, (With<PendingOffering>, Without<shared::casting::CastState>)>()
        .iter(world)
        .collect();
    for player in stopped {
        cancel_pickup(world, player);
    }
}

fn prepare_avatars(world: &mut World) {
    let avatars: Vec<_> = world
        .query::<(Entity, &Npc, &InMap, Option<&AvatarPrepared>)>()
        .iter(world)
        .filter(|(_, npc, map, ready)| {
            npc.template_id == AVATAR && map.map_id == MAP && ready.is_none()
        })
        .map(|(entity, _, _, _)| entity)
        .collect();
    for avatar in avatars {
        world.entity_mut(avatar).insert(AvatarPrepared);
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: avatar,
                target: avatar,
                spell_id: FEAST,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
}

pub(crate) fn all_avatars_fed(world: &mut World, map: InMap) -> bool {
    world
        .query::<(&Npc, &InMap, &AvatarFed)>()
        .iter(world)
        .filter(|(npc, other, _)| npc.template_id == AVATAR && **other == map)
        .count()
        >= 6
}

fn deliver(world: &mut World, player: Entity, avatar: Entity, map: InMap) {
    crate::auras::remove_auras(
        world,
        player,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == CARRY,
    );
    world.entity_mut(player).remove::<CarryingOffering>();
    crate::auras::remove_auras(
        world,
        avatar,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == FEAST,
    );
    if world
        .get::<shared::casting::CastState>(avatar)
        .is_some_and(|cast| cast.spell_id == FEAST)
    {
        crate::spell_cast::cancel_cast(world, avatar);
    }
    world.entity_mut(avatar).insert(AvatarFed);
    if !all_avatars_fed(world, map) {
        return;
    }
    let boss = world
        .query::<(Entity, &Npc, &InMap)>()
        .iter(world)
        .find_map(|(entity, npc, other)| {
            (npc.template_id == HOARDMONGER && *other == map).then_some(entity)
        });
    if let Some(boss) = boss {
        if let Some(mut flags) = world.get_mut::<UnitFlags>(boss) {
            flags.0 &= !(UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC);
            crate::spell_cast::mark_dirty(world, boss);
        }
    }
}
