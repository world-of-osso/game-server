//! Pure action metadata from the same programs and bindings used by the rotations.
use crate::class_progression::TraitLoadout;
use crate::profiles::profile;
use bevy::prelude::{App, Entity, Resource, World};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CacheKey {
    spec: u32,
    flamestrike: bool,
    infusion: bool,
    feral_berserk: Option<u32>,
    balance_alignment: u32,
    lunar_calling: bool,
    pet_summon: Option<u32>,
    coatings: Vec<u32>,
}

struct CachedActions {
    program: server::simc_apl::Program,
    spells: Vec<u32>,
}

#[derive(Resource, Default)]
struct ActionCache(BTreeMap<CacheKey, Arc<CachedActions>>);

/// Register the pure metadata provider before spawning class followers.
pub fn install_provider(app: &mut App) {
    app.init_resource::<ActionCache>()
        .insert_resource(server::followers::FollowerActionProvider {
            before_passives: configure_before_passives,
            source_actions: provide_source_actions,
            equipped_actions: provide_equipped_actions,
        });
}

fn configure_before_passives(world: &mut World, actor: Entity, spec: u32) -> Result<(), String> {
    if spec == 64 {
        crate::frost_mage_rotation::configure_actor_reactions(world, actor)?;
    }
    Ok(())
}

fn cached_actions(
    world: &mut World,
    actor: Entity,
    spec: u32,
) -> Result<Arc<CachedActions>, String> {
    let input = read_inputs(world, actor, spec)?;
    let key = CacheKey {
        spec,
        flamestrike: input
            .traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 2120),
        infusion: input
            .traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1241059),
        feral_berserk: input.feral_berserk,
        balance_alignment: input.balance_alignment,
        lunar_calling: input.lunar_calling,
        pet_summon: input.pet_summon,
        coatings: input.coatings.clone(),
    };
    let cached = world
        .get_resource::<ActionCache>()
        .ok_or("class follower action metadata cache unavailable")?
        .0
        .get(&key)
        .cloned();
    let cached = match cached {
        Some(cached) => cached,
        None => {
            let (program, bindings) = parse(spec, &input)?;
            let spells = crate::literal_rotation::program_action_spells_with_composite(
                &program,
                &bindings,
                "apply_poison",
                input.coatings.iter().copied(),
            );
            let cached = Arc::new(CachedActions { program, spells });
            world
                .resource_mut::<ActionCache>()
                .0
                .insert(key, Arc::clone(&cached));
            cached
        }
    };
    Ok(cached)
}

fn provide_source_actions(world: &mut World, actor: Entity, spec: u32) -> Result<Vec<u32>, String> {
    Ok(cached_actions(world, actor, spec)?.spells.clone())
}

fn provide_equipped_actions(
    world: &mut World,
    actor: Entity,
    spec: u32,
) -> Result<Vec<u32>, String> {
    let cached = cached_actions(world, actor, spec)?;
    let mut bindings = crate::simc_apl_native::Bindings::default();
    equipped_action_spells(world, actor, &cached.program, &mut bindings)
}

/// Actor-specific inputs selected before pure program parsing.
pub struct ActionInputs<'a> {
    pub traits: &'a TraitLoadout,
    pub feral_berserk: Option<u32>,
    pub balance_alignment: u32,
    pub lunar_calling: bool,
    pub pet_summon: Option<u32>,
    pub coatings: Vec<u32>,
}

/// Read selected class actions without changing the actor or world.
pub fn read_inputs(world: &World, actor: Entity, spec: u32) -> Result<ActionInputs<'_>, String> {
    let traits = world
        .get::<TraitLoadout>(actor)
        .ok_or("follower traits unavailable")?;
    let feral_berserk = if spec == 103 {
        Some(
            crate::spell_cast::knowledge::resolve_action(world, actor, 106951)?
                .ok_or("native Feral reference lacks its selected Berserk/Incarnation action")?,
        )
    } else {
        None
    };
    let (balance_alignment, lunar_calling) = if spec == 102 {
        crate::balance_rotation::bindings::read_selection(world, actor)?
    } else {
        (194223, false)
    };
    let pet_summon = if matches!(spec, 265..=267) {
        crate::literal_rotation::default_pet::ConfiguredPet::read(world, actor, &profile(spec)?)?
            .map(|pet| pet.spell_id())
    } else {
        None
    };
    let coatings = if matches!(spec, 259 | 261) {
        crate::assassination_rotation::read_coatings(world, actor)
            .into_iter()
            .map(|action| action.spell)
            .collect()
    } else {
        Vec::new()
    };
    Ok(ActionInputs {
        traits,
        feral_berserk,
        balance_alignment,
        lunar_calling,
        pet_summon,
        coatings,
    })
}

/// Parse the program and its bindings without initializing the actor.
pub fn parse(
    spec: u32,
    input: &ActionInputs<'_>,
) -> Result<(server::simc_apl::Program, crate::simc_apl_native::Bindings), String> {
    use crate::*;
    match spec {
        62 => Ok((
            arcane_rotation::program(&profile(spec)?)?,
            arcane_rotation::bindings(),
        )),
        63 => Ok((
            fire_rotation::source::load(&profile(spec)?)?,
            fire_rotation::bindings::build_for_traits(input.traits),
        )),
        64 => Ok((
            frost_mage_rotation::program(&profile(spec)?)?,
            frost_mage_rotation::bindings::build(),
        )),
        65 => Ok((
            holy_paladin_rotation::program()?,
            holy_paladin_rotation::bindings(),
        )),
        66 => Ok((
            protection_paladin_rotation::program(&profile(spec)?.options["actions"])?,
            protection_paladin_rotation::context::bindings(),
        )),
        70 => Ok((ret_rotation::read_program()?, ret_rotation::bindings())),
        71 => Ok((
            arms_rotation::program(&profile(spec)?)?,
            arms_rotation::bindings(),
        )),
        72 => Ok((fury_rotation::program()?, fury_rotation::bindings())),
        73 => Ok((
            protection_warrior_rotation::program(&profile(spec)?)?,
            protection_warrior_rotation::bindings(),
        )),
        102 => Ok((
            balance_rotation::read_program(&profile(spec)?)?,
            balance_rotation::bindings::build_for_actions(
                input.balance_alignment,
                input.lunar_calling,
            ),
        )),
        103 => Ok((
            feral_rotation::source::load()?,
            feral_rotation::bindings::build_for_berserk(
                input
                    .feral_berserk
                    .ok_or("Feral selected action unavailable")?,
            ),
        )),
        104 => Ok((
            guardian_tank_rotation::program()?,
            guardian_tank_rotation::bindings(),
        )),
        105 => Ok((
            restoration_druid_rotation::program()?,
            restoration_druid_rotation::bindings(),
        )),
        250 => Ok((
            blood_tank_rotation::program()?,
            blood_tank_rotation::bindings(),
        )),
        251 => Ok((
            frost_rotation::read_program(&profile(spec)?)?,
            frost_rotation::bindings::build(),
        )),
        252 => Ok((
            unholy_rotation::load_program(&profile(spec)?)?,
            unholy_rotation::bindings(),
        )),
        253 => Ok((
            beast_mastery_rotation::program()?,
            beast_mastery_rotation::bindings::build(),
        )),
        254 => Ok((
            marksmanship_rotation::program()?,
            marksmanship_rotation::bindings::build(),
        )),
        255 => Ok((
            survival_rotation::program()?,
            survival_rotation::bindings::build(),
        )),
        256 => Ok((
            discipline_priest_rotation::program()?,
            discipline_priest_rotation::bindings(),
        )),
        257 => Ok((
            holy_priest_rotation::program()?,
            holy_priest_rotation::bindings(),
        )),
        258 => Ok((
            shadow_rotation::program()?,
            shadow_rotation::bindings::build(),
        )),
        259 => Ok((
            assassination_rotation::program(&assassination_rotation::action_source(&profile(
                spec,
            )?)?)?,
            assassination_rotation::bindings(),
        )),
        260 => Ok((
            outlaw_rotation::load_program(&profile(spec)?)?,
            outlaw_rotation::bindings(),
        )),
        261 => Ok((
            subtlety_rotation::load_program(&profile(spec)?)?,
            subtlety_rotation::bindings(),
        )),
        262 => Ok((
            elemental_rotation::read_program(&profile(spec)?)?,
            elemental_rotation::bindings::build(),
        )),
        263 => Ok((
            enhancement_rotation::read_program()?,
            enhancement_rotation::bindings::build(),
        )),
        264 => Ok((
            restoration_shaman_rotation::program()?,
            restoration_shaman_rotation::bindings(),
        )),
        265..=267 => {
            let mut bindings = match spec {
                265 => affliction_rotation::bindings::build(),
                266 => demonology_rotation::bindings::build(),
                267 => destruction_rotation::bindings::build(),
                _ => unreachable!(),
            };
            if let Some(spell) = input.pet_summon {
                if bindings
                    .actions
                    .get("summon_pet")
                    .is_some_and(|&existing| existing != spell)
                {
                    return Err(
                        "configured default pet conflicts with summon_pet native binding".into(),
                    );
                }
                bindings.actions.insert("summon_pet".into(), spell);
            }
            let profile = profile(spec)?;
            let program = match spec {
                265 => affliction_rotation::read_program(&profile)?,
                266 => demonology_rotation::read_program(&profile)?,
                267 => destruction_rotation::read_program(&profile)?,
                _ => unreachable!(),
            };
            Ok((program, bindings))
        }
        268 => Ok((
            brewmaster_rotation::program(&profile(spec)?)?,
            brewmaster_rotation::bindings_for_infusion(
                input
                    .traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == 1241059),
            ),
        )),
        269 => Ok((
            windwalker_rotation::source::load()?,
            windwalker_rotation::bindings::build(),
        )),
        270 => Ok((
            mistweaver_rotation::program()?,
            mistweaver_rotation::bindings(),
        )),
        577 => Ok((
            havoc_rotation::load_program()?,
            havoc_rotation::bindings::build(),
        )),
        581 => Ok((
            vengeance_tank_rotation::program()?,
            vengeance_tank_rotation::bindings(),
        )),
        1467 => Ok((
            devastation_rotation::source::load()?,
            devastation_rotation::bindings::build(),
        )),
        1468 => Ok((
            preservation_evoker_rotation::program()?,
            preservation_evoker_rotation::bindings(),
        )),
        1480 => Ok((
            devourer_rotation::load_program()?,
            devourer_rotation::bindings::build(),
        )),
        _ => Err(format!("spec{spec} has no pure follower action metadata")),
    }
}

/// Resolve the equipped on-use spells that this program can select.
pub fn equipped_action_spells(
    world: &World,
    actor: Entity,
    program: &server::simc_apl::Program,
    bindings: &mut crate::simc_apl_native::Bindings,
) -> Result<Vec<u32>, String> {
    let equipment = crate::literal_rotation::equipment::Equipment::load(world, actor, bindings)?;
    let mut spells = Vec::new();
    for (_, action) in program.actions() {
        if let Some(slot) = equipment.slots.get(&action.name)
            && let Some(item) = slot.on_use
        {
            spells.push(item.on_use.spell_id);
        }
        let slots = match action.name.as_str() {
            "use_item" => equipment.slot_for_action(action)?.into_iter().collect(),
            "use_items" => equipment.use_items_slots(action, program)?,
            _ => continue,
        };
        for slot in slots {
            if let Some(item) = equipment.slots[slot].on_use {
                spells.push(item.on_use.spell_id);
            }
        }
    }
    spells.sort_unstable();
    spells.dedup();
    Ok(spells)
}

/// Read a class's program-bound native action IDs without initializing its runtime.
pub fn action_spells(spec: u32, input: &ActionInputs<'_>) -> Result<Vec<u32>, String> {
    let (program, bindings) = parse(spec, input)?;
    Ok(
        crate::literal_rotation::program_action_spells_with_composite(
            &program,
            &bindings,
            "apply_poison",
            input.coatings.iter().copied(),
        ),
    )
}
