//! SimC76f8118 unique_gear_midnight.cpp3657–3743: Gebbo's Bottomless Bag picks
//! one of six artifacts per proc. The artifact rows carry no values; each
//! rating comes from the item-scaled driver effect named in the SimC source.
use bevy::prelude::*;

use super::{
    ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};
use crate::spell_info::SpellStore;

const DRIVER: u32 = 1292291;
const SALMON: u32 = 1308013;
const GRALSTONE: u32 = 1308012;
const VOIDFIN: u32 = 1308014;
const SCROLL: u32 = 1306870;
const SEASHELL: u32 = 1292299;
const TOTEM: u32 = 1292300;
/// SimC artifact order; one is chosen uniformly per proc.
const ARTIFACTS: [u32; 6] = [SALMON, GRALSTONE, VOIDFIN, SCROLL, SEASHELL, TOTEM];
/// SimC `default_ticks`: scroll and seashell stack limit and 1s steps.
const TICKS: u8 = 12;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        DRIVER,
        AuraScript {
            on_proc: Some(apply_random_artifact),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SCROLL,
        AuraScript {
            on_periodic: Some(drain_scroll),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SEASHELL,
        AuraScript {
            on_periodic: Some(ramp_seashell),
            ..Default::default()
        },
    );
    scripts.0.insert(
        TOTEM,
        AuraScript {
            on_proc: Some(drain_totem),
            ..Default::default()
        },
    );
}

/// Item-scaled driver effect (SimC `effectN(index + 1).average(effect)`).
fn read_driver_value(world: &World, unit: Entity, driver: &AuraInstance, index: u8) -> Option<f32> {
    let mut spell = world.resource::<SpellStore>().get(DRIVER)?.clone();
    spell.cast_item_level = driver.cast_item_level;
    spell.cast_item_inventory_type = driver.cast_item_inventory_type;
    let effect = spell.effects.iter().find(|effect| effect.index == index)?;
    Some(crate::spell_cast::effects::modded_base_points(
        world, unit, &spell, effect,
    ))
}

/// One artifact application: per-stack rating, applied effect mask and stacks.
struct ArtifactGrant {
    rating: f32,
    rating_effects: &'static [u8],
    effect_mask: u32,
    stacks: u8,
    max_stacks: u8,
}

/// unique_gear_midnight.cpp3668–3716. Only rating effects apply, plus the native
/// 1s periodic that steps scroll/seashell stacks; the seashell self-dot is ignored.
fn read_artifact_grant(
    world: &World,
    unit: Entity,
    driver: &AuraInstance,
    artifact: u32,
    previous_stacks: u8,
) -> Option<ArtifactGrant> {
    let value = |index| read_driver_value(world, unit, driver, index);
    let single = |rating, effect_mask| ArtifactGrant {
        rating,
        rating_effects: &[0],
        effect_mask,
        stacks: 1,
        max_stacks: 1,
    };
    let all_secondaries = |rating| ArtifactGrant {
        rating,
        rating_effects: &[0, 1, 2, 3],
        effect_mask: 0b1111,
        stacks: 1,
        max_stacks: 1,
    };
    Some(match artifact {
        SALMON => all_secondaries(value(5)?),
        VOIDFIN => all_secondaries(-value(6)?),
        GRALSTONE => single(value(3)?, 0b1),
        // Reverse buff: a trigger restores all stacks, each tick removes one.
        SCROLL => ArtifactGrant {
            stacks: TICKS,
            max_stacks: TICKS,
            ..single(value(1)? / f32::from(TICKS), 0b11)
        },
        // Ramping buff: a trigger adds one stack, each tick adds one.
        SEASHELL => ArtifactGrant {
            stacks: previous_stacks.saturating_add(1).min(TICKS),
            max_stacks: TICKS,
            ..single(value(0)?, 0b11)
        },
        // Banked buff: a trigger restores its initial stacks, each hit drains one.
        TOTEM => {
            let decrement = value(7)?;
            let stacks = (value(2)? / decrement)
                .round()
                .clamp(1.0, f32::from(u8::MAX)) as u8;
            ArtifactGrant {
                stacks,
                max_stacks: stacks,
                ..single(decrement, 0b1)
            }
        }
        _ => return None,
    })
}

fn read_owned_stacks(world: &World, unit: Entity, spell_id: u32) -> u8 {
    world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell_id && aura.caster == unit)
        })
        .map_or(0, |aura| aura.stacks)
}

fn apply_random_artifact(
    world: &mut World,
    unit: Entity,
    driver: &AuraInstance,
    _other: Option<Entity>,
) {
    let artifact = ARTIFACTS[(crate::random::random::<f64>() * ARTIFACTS.len() as f64) as usize];
    let previous = read_owned_stacks(world, unit, artifact);
    let Some(grant) = read_artifact_grant(world, unit, driver, artifact, previous) else {
        error!(
            ?unit,
            artifact, "Gebbo's Bottomless Bag lacks native driver values"
        );
        return;
    };
    let Some(mut spell) = world.resource::<SpellStore>().get(artifact).cloned() else {
        error!(?unit, artifact, "native Gebbo artifact absent");
        return;
    };
    for effect in spell
        .effects
        .iter_mut()
        .filter(|effect| grant.rating_effects.contains(&effect.index))
    {
        effect.base_points = grant.rating;
    }
    super::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: artifact,
            effect_mask: grant.effect_mask,
            source: AuraSource::Cast,
        },
        &spell,
    );
    set_artifact_stacks(world, unit, artifact, grant.stacks, Some(grant.max_stacks));
}

fn set_artifact_stacks(
    world: &mut World,
    unit: Entity,
    spell_id: u32,
    stacks: u8,
    maximum: Option<u8>,
) {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    let Some(aura) = auras
        .auras
        .iter_mut()
        .find(|aura| aura.spell_id == spell_id && aura.caster == unit)
    else {
        return;
    };
    if let Some(maximum) = maximum {
        aura.max_stacks = maximum;
    }
    aura.stacks = stacks.min(aura.max_stacks.max(1));
    auras.dirty = true;
    drop(auras);
    super::auras_changed(world, unit);
}

/// Decrement one stack; the last decrement expires the buff (SimC `decrement`).
fn remove_one_stack(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.stacks > 1 {
        set_artifact_stacks(world, unit, aura.spell_id, aura.stacks - 1, None);
        return;
    }
    let (spell_id, instance) = (aura.spell_id, aura.instance_id);
    super::remove_auras(world, unit, RemoveReason::Expired, |live| {
        live.spell_id == spell_id && live.instance_id == instance
    });
}

fn drain_scroll(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if effect_index == 1 {
        remove_one_stack(world, unit, aura);
    }
}

fn ramp_seashell(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if effect_index == 1 {
        set_artifact_stacks(world, unit, SEASHELL, aura.stacks.saturating_add(1), None);
    }
}

/// unique_gear_midnight.cpp3718–3738: every hit drains one totem stack, behind the
/// totem's native proc recovery.
fn drain_totem(world: &mut World, unit: Entity, aura: &AuraInstance, _other: Option<Entity>) {
    remove_one_stack(world, unit, aura);
}
