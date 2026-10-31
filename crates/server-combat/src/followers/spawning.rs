use bevy::prelude::*;
use shared::components::{Level, Mana, UnitFactionTemplate, UnitFlags, UnitLevel, UnitSummonedBy};

use super::{Follower, Profile};
use crate::spell_cast::powers::PowerPools;

pub(super) fn spawn_members(
    world: &mut World,
    owner: Entity,
    profiles: &[Profile],
) -> Result<Vec<Entity>, String> {
    let mut members = Vec::with_capacity(profiles.len());
    for (slot, profile) in profiles.iter().enumerate() {
        match spawn(world, owner, profile.clone()) {
            Ok(unit) => {
                world
                    .entity_mut(unit)
                    .insert(crate::creature_ai::PartyFollowSlot::new(
                        slot,
                        profiles.len(),
                    ));
                members.push(unit);
            }
            Err(error) => {
                unsummon_members(world, &members);
                return Err(error);
            }
        }
    }
    Ok(members)
}

pub(super) fn unsummon_members(world: &mut World, members: &[Entity]) {
    for &unit in members {
        crate::zones::unsummon(world, unit);
    }
}

pub(super) fn spawn(world: &mut World, owner: Entity, profile: Profile) -> Result<Entity, String> {
    let position = *world
        .get::<shared::components::Position>(owner)
        .ok_or("follower owner has no position")?;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let unit = crate::zones::summon_creature(world, owner, profile.entry, [x, y, z, 0.0])
        .ok_or_else(|| format!("authentic follower template {} unavailable", profile.entry))?;
    if let Err(error) = initialize(world, owner, unit, profile) {
        world.despawn(unit);
        return Err(error);
    }
    Ok(unit)
}

pub(super) fn initialize(
    world: &mut World,
    owner: Entity,
    unit: Entity,
    profile: Profile,
) -> Result<(), String> {
    let level = world
        .get::<Level>(owner)
        .ok_or("follower owner has no level")?
        .0;
    let faction = *world
        .get::<UnitFactionTemplate>(owner)
        .ok_or("follower owner has no faction")?;
    let flags = UnitFlags(
        world.get::<UnitFlags>(unit).map_or(0, |flags| flags.0)
            | crate::faction::UNIT_FLAG_PLAYER_CONTROLLED,
    );
    let (stats, pools, _) = class_stats(world, &profile, level)?;
    let (known, traits) = crate::class_progression::evaluate_npc_starter(
        world.resource::<crate::spell_info::ClassSpells>(),
        profile.class_id,
        profile.spec_id,
        level,
    )?;
    let passives: Vec<_> = known
        .iter()
        .filter(|spell| spell.passive)
        .map(|spell| spell.spell_id)
        .collect();
    let spec_id = profile.spec_id;
    let armor = derive_creature_armor(world, unit, profile.entry, level)?;
    let mana = Mana {
        current: stats.base_mana,
        max: stats.base_mana,
    };
    let health = shared::components::Health {
        current: stats.max_health,
        max: stats.max_health,
    };
    let form = profile.form_spell;
    // The ordinary StatsPlugin query owns derived-stat refresh after passive
    // StatModifiers change, just as for players. SheetOwner remains absent.
    let stat_fields = (stats.primary, stats.ratings, stats.derived());
    world
        .entity_mut(unit)
        .remove::<crate::creature_aggro::AggroRange>()
        .insert((
            Follower { owner, profile },
            stat_fields,
            stats,
            pools,
            mana,
            health,
            UnitLevel(level),
            Level(level),
            armor,
            faction,
            flags,
            UnitSummonedBy(owner.to_bits()),
            crate::creature_ai::FollowOwner::new(owner),
        ));
    // Native class knowledge is independent of controller rows; the unit
    // remains an NPC even when no kit has been published for its requested spec.
    world.entity_mut(unit).insert((
        shared::components::ActiveSpec(spec_id),
        crate::class_progression::KnownSpells::from_baseline(
            known.iter().map(|spell| spell.spell_id).collect(),
        ),
        traits,
    ));
    crate::auras::sync_passives(world, unit, crate::auras::AuraSource::KnownSpell, &passives);
    if form != 0 {
        crate::spell_cast::creature_cast(world, unit, form, Some(unit), true)
            .map_err(|error| format!("follower preparation spell {form} failed: {error:?}"))?;
    }
    Ok(())
}

fn derive_creature_armor(
    world: &World,
    unit: Entity,
    entry: u32,
    level: u8,
) -> Result<crate::level_scaling::CreatureArmor, String> {
    let difficulty = world
        .get::<crate::zones::CreatureDifficulty>(unit)
        .map_or(0, |difficulty| difficulty.0);
    world
        .get_resource::<crate::world_data::WorldData>()
        .and_then(|data| data.creature_armor_at(entry, difficulty, level))
        .map(crate::level_scaling::CreatureArmor)
        .ok_or_else(|| {
            format!("follower {entry} level{level} difficulty{difficulty} lacks native armor data")
        })
}

pub(super) fn class_stats(
    world: &World,
    profile: &Profile,
    level: u8,
) -> Result<(crate::stats::CharacterStats, PowerPools, Vec<u32>), String> {
    super::class_spawn::class_stats(world, profile, level, 0)
}
