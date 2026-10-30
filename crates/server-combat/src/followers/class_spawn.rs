//! Summon a player-class companion using a neutral, kit-free creature template.
#[cfg(test)]
#[path = "class_spawn_tests.rs"]
mod tests;
use bevy::prelude::*;
use lightyear::prelude::Replicate;
use shared::components::{
    CharacterAppearance, CharacterVisual, Level, Mana, UnitFactionTemplate, UnitFlags, UnitLevel,
    UnitSummonedBy,
};

use super::{Follower, Profile, class_profiles::ClassProfile};

// Runtime-only identity. It is not a story creature or a catalog creature kit.
const CLASS_FOLLOWER_ENTRY: u32 = u32::MAX - 1;

/// Normal dungeons use Journal gear. Keystone bands are not yet implemented.
#[derive(Clone, Copy)]
pub enum GearSource {
    Journal,
}

fn neutral_template(name: &str, level: u8) -> crate::world_data::CreatureTemplate {
    use crate::world_data::CreatureTemplate;
    CreatureTemplate {
        entry: CLASS_FOLLOWER_ENTRY,
        name: name.to_owned(),
        subname: String::new(),
        creature_type: 7,
        family: 0,
        type_flags: 0,
        flags_extra: 0,
        static_flags: [0; 8],
        min_level: level,
        max_level: level,
        faction: 0,
        npc_flags: 0,
        speed_walk: 1.0,
        speed_run: 1.0,
        base_attack_time_ms: 2000,
        health_modifier: 1.0,
        unit_class: 1,
        expansion: 0,
        rank: 0,
        experience_modifier: 0.0,
        gives_experience: false,
        script_name: String::new(),
        aggro_mode: crate::creature_aggro::AggroMode::Reactive,
        trigger: false,
        unit_flags: 0,
        unit_flags2: 0,
        models: Vec::new(),
        tuning: None,
        armor: 0.0,
        armor_modifier: 0.0,
        damage_modifier: 1.0,
        health_scaling_expansion: 0,
        mana_modifier: 1.0,
        regenerates_power: true,
        regenerates_health: true,
    }
}

fn is_melee(spec: u32) -> Result<bool, String> {
    match spec {
        65 | 66 | 70 | 71 | 72 | 73 | 103 | 104 | 250 | 251 | 252 | 255 | 259 | 260 | 261 | 263
        | 268 | 269 | 270 | 577 | 581 | 1480 => Ok(true),
        62 | 63 | 64 | 102 | 105 | 253 | 254 | 256 | 257 | 258 | 262 | 264 | 265 | 266 | 267
        | 1467 | 1468 => Ok(false),
        _ => Err(format!(
            "follower spec {spec} has no movement classification"
        )),
    }
}

pub(super) fn profile_for(
    source: &ClassProfile,
    slot: usize,
    attack_spell: u32,
) -> Result<Profile, String> {
    Ok(Profile {
        entry: CLASS_FOLLOWER_ENTRY,
        name: format!("Follower{}", slot + 1),
        class_id: source.class_id,
        spec_id: source.spec_id,
        role: source.group_role()?,
        attack_spell,
        cooldown_spell: 0,
        heal_spell: 0,
        taunt_spell: 0,
        form_spell: 0,
        melee: is_melee(source.spec_id)?,
    })
}

pub(super) fn attack_spell(
    classes: &crate::spell_info::ClassSpells,
    store: &crate::spell_info::SpellStore,
    source: &ClassProfile,
    known: &[crate::class_progression::KnownSpell],
) -> Result<u32, String> {
    if is_melee(source.spec_id)? {
        return Ok(0);
    }
    let spec = classes
        .classes
        .iter()
        .find(|class| class.class_id == source.class_id)
        .and_then(|class| {
            class
                .specs
                .iter()
                .find(|spec| spec.spec_id == source.spec_id)
        })
        .ok_or("follower specialization unavailable")?;
    spec.spells
        .iter()
        .filter(|spell| {
            !spell.passive && known.iter().any(|known| known.spell_id == spell.spell_id)
        })
        .filter_map(|spell| store.get(spell.spell_id))
        .filter(|spell| {
            !spell.effects.iter().any(|effect| {
                matches!(
                    effect.effect,
                    crate::death::resurrection::EFFECT_RESURRECT
                        | crate::death::resurrection::EFFECT_RESURRECT_WITH_AURA
                )
            })
        })
        .find(|spell| {
            spell
                .misc
                .as_ref()
                .and_then(|misc| misc.range)
                .is_some_and(|range| range.max_hostile.max(range.max_friendly) >= 20.0)
        })
        .map(|spell| spell.id)
        .ok_or_else(|| {
            format!(
                "follower spec {} has no known ranged spec spell",
                source.spec_id
            )
        })
}

fn spawn_neutral(
    world: &mut World,
    owner: Entity,
    name: &str,
    level: u8,
) -> Result<Entity, String> {
    let map = *world
        .get::<crate::maps::InMap>(owner)
        .ok_or("follower owner has no map")?;
    let position = *world
        .get::<shared::components::Position>(owner)
        .ok_or("follower owner has no position")?;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let zone = world
        .get::<shared::components::Zone>(owner)
        .map_or(0, |zone| zone.id as u16);
    let replicate = world.get::<Replicate>(owner).is_some();
    let difficulty_id = crate::zones::map_difficulty_id(world, map);
    let spawn = crate::world_data::CreatureSpawn {
        guid: 0,
        template_entry: CLASS_FOLLOWER_ENTRY,
        map: map.map_id as u16,
        zone_id: zone,
        position_x: x,
        position_y: y,
        position_z: z,
        orientation: 0.0,
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: 0,
        display_id: Some(0),
        unit_flags: None,
        spawn_difficulties: Vec::new(),
        equipment_id: 0,
    };
    let template = neutral_template(name, level);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let entity = world.resource_scope(|world, data: Mut<crate::world_data::WorldData>| {
        let mut commands = Commands::new(&mut queue, world);
        let mut context = crate::zones::CreatureSpawnContext {
            map,
            difficulty_id,
            ground: None,
            temporary: true,
        };
        let entity = crate::zones::spawn_creature(
            &mut commands,
            &data,
            &template,
            &mut context,
            replicate,
            &spawn,
            &mut rand::thread_rng(),
        )?;
        commands
            .entity(entity)
            .insert((
                crate::zones::TempSummon { summoner: owner },
                crate::zones::CreatureDifficulty(difficulty_id),
            ))
            .remove::<crate::combat::RespawnDelay>();
        Some(entity)
    });
    queue.apply(world);
    entity.ok_or_else(|| "neutral class follower spawn refused".to_owned())
}

/// Spawn one non-player class follower with the selected real gear source.
pub fn spawn_class(
    world: &mut World,
    owner: Entity,
    source: &ClassProfile,
    sex: u8,
    slot: usize,
    gear: GearSource,
) -> Result<Entity, String> {
    if let Some(error) = &source.rejection {
        return Err(format!(
            "follower spec {} rejected: {error}",
            source.spec_id
        ));
    }
    let level = world
        .get::<Level>(owner)
        .ok_or("follower owner has no level")?
        .0;
    let faction = *world
        .get::<UnitFactionTemplate>(owner)
        .ok_or("follower owner has no faction")?;
    let classes = world
        .get_resource::<crate::spell_info::ClassSpells>()
        .ok_or("follower classes unavailable")?;
    let (known, traits, talent_source) = crate::class_progression::evaluate_follower_progression(
        classes,
        source.class_id,
        source.race,
        source.spec_id,
        level,
        source.talent_source()?,
        source
            .talent_entries
            .as_ref()
            .ok_or("follower talents unavailable")?,
    )?;
    let store = world
        .get_resource::<crate::spell_info::SpellStore>()
        .ok_or("follower spell store unavailable")?;
    let attack_spell = attack_spell(classes, store, source, &known)?;
    let profile = profile_for(source, slot, attack_spell)?;
    let (stats, pools, _) =
        super::spawning::class_stats_for_race(world, &profile, level, source.race)?;
    let entity = spawn_neutral(world, owner, &profile.name, level)?;
    let passives: Vec<_> = known
        .iter()
        .filter(|spell| spell.passive)
        .map(|spell| spell.spell_id)
        .collect();
    let stat_fields = (stats.primary, stats.ratings, stats.derived());
    world
        .entity_mut(entity)
        .remove::<shared::components::ModelDisplay>()
        .remove::<crate::creature_aggro::AggroRange>()
        .remove::<crate::level_scaling::CreatureArmor>()
        .insert((
            Follower { owner, profile },
            super::AplControlled,
            CharacterVisual {
                race: source.race,
                class: source.class_id,
                appearance: CharacterAppearance {
                    sex,
                    ..Default::default()
                },
            },
            stat_fields,
            stats,
            pools,
            Mana {
                current: stats.base_mana,
                max: stats.base_mana,
            },
            shared::components::Health {
                current: stats.max_health,
                max: stats.max_health,
            },
            UnitLevel(level),
            Level(level),
            faction,
            UnitFlags(crate::faction::UNIT_FLAG_PLAYER_CONTROLLED),
            UnitSummonedBy(owner.to_bits()),
            crate::creature_ai::FollowOwner::new(owner),
        ))
        .insert((
            shared::components::ActiveSpec(source.spec_id),
            crate::class_progression::KnownSpells::from_baseline(
                known.iter().map(|spell| spell.spell_id).collect(),
            ),
            traits,
        ));
    crate::auras::sync_passives(
        world,
        entity,
        crate::auras::AuraSource::KnownSpell,
        &passives,
    );
    let profile = world
        .get::<Follower>(entity)
        .expect("spawned class follower")
        .profile
        .clone();
    let equipped = match gear {
        GearSource::Journal => super::gear::equip(world, entity, &profile),
    }
    .and_then(|()| super::gear::publish_appearance(world, entity));
    if let Err(error) = equipped {
        super::spawning::unsummon_members(world, &[entity]);
        return Err(error);
    }
    info!(
        "class follower spec {} uses {}",
        source.spec_id,
        talent_source.label()
    );
    Ok(entity)
}
