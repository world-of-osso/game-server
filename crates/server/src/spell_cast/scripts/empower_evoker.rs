//! TC a352b1fa spell_evoker.cpp356–416; SimC76f8118 sc_evoker.cpp5265/5507.
//! Every coefficient and stage duration remains native70448 metadata.
use bevy::prelude::*;

use crate::auras::ActiveAuras;
use crate::spell_cast::CastFailure;
use crate::spell_info::{SpellInfo, SpellStore};

const FIRE_DAMAGE: u32 = 357209;
const SURGE_DAMAGE: u32 = 359077;
const BLAST_FURNACE: u32 = 375510;
const SCOURING_FLAME: u32 = 378438;
const ETERNITYS_SPAN: u32 = 375757;
const ANIMOSITY: u32 = 375797;
const DRAGONRAGE: u32 = 375087;
const BASE_FIRE_TICKS: u32 = 10;
const CONVERTED_TICKS_PER_RANK: u32 = 3;

#[derive(Component)]
struct AnimosityProgress {
    dragonrage_instance: u32,
    effectiveness: f32,
}

/// SimC4648–4681: each successful empower extends the same Dragonrage less.
fn extend_dragonrage(world: &mut World, caster: Entity) {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return;
    };
    let Some(passive) = auras.iter().find(|aura| aura.spell_id == ANIMOSITY) else {
        return;
    };
    let extension = passive
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Animosity extension effect0")
        .amount;
    let reduction = passive
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Animosity effectiveness effect1")
        .amount;
    let Some(rage) = auras
        .iter()
        .find(|aura| aura.spell_id == DRAGONRAGE && aura.caster == caster)
    else {
        return;
    };
    let instance = rage.instance_id;
    let effectiveness = world
        .get::<AnimosityProgress>(caster)
        .filter(|progress| progress.dragonrage_instance == instance)
        .map_or(1.0, |progress| progress.effectiveness);
    crate::auras::extend_owned_duration(
        world,
        caster,
        caster,
        &[DRAGONRAGE],
        (extension * effectiveness) as u32,
    );
    world.entity_mut(caster).insert(AnimosityProgress {
        dragonrage_instance: instance,
        effectiveness: effectiveness * (1.0 - reduction / 100.0),
    });
}

fn read_native_aura_amount(world: &World, caster: Entity, spell: u32) -> f32 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == spell))
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount)
}

fn filter_scouring_effect(world: &World, caster: Entity, child: &mut SpellInfo) {
    // TC648–675 gates the dispel; native70448 moved it to effect5 and fixes its count at2.
    if !crate::auras::has_aura(world, caster, SCOURING_FLAME) {
        child
            .effects
            .retain(|effect| effect.effect != crate::spell_cast::effects::EFFECT_DISPEL);
    }
}

fn build_fire_breath_release(world: &World, caster: Entity, rank: u8) -> SpellInfo {
    let mut child = world
        .resource::<SpellStore>()
        .get(FIRE_DAMAGE)
        .expect("Fire Breath requires native357209 release metadata")
        .clone();
    let period_ms = child
        .effects
        .iter()
        .find(|effect| effect.index == 1 && effect.aura == 3)
        .expect("native Fire Breath periodic effect1")
        .aura_period_ms;
    let bonus_ticks =
        read_native_aura_amount(world, caster, BLAST_FURNACE) / (period_ms as f32 / 1000.0);
    let ticks =
        BASE_FIRE_TICKS - CONVERTED_TICKS_PER_RANK * u32::from(rank - 1) + bonus_ticks as u32;
    let duration = child
        .misc
        .as_mut()
        .and_then(|misc| misc.duration.as_mut())
        .expect("native Fire Breath duration");
    duration.duration_ms = (ticks * period_ms) as i32;
    duration.max_duration_ms = duration.duration_ms;
    child.cast_empower_level = Some(rank);
    filter_scouring_effect(world, caster, &mut child);
    child
}

fn build_eternity_surge_release(world: &World, caster: Entity, rank: u8) -> SpellInfo {
    let mut child = world
        .resource::<SpellStore>()
        .get(SURGE_DAMAGE)
        .expect("Eternity Surge requires native359077 release metadata")
        .clone();
    let span = crate::auras::has_aura(world, caster, ETERNITYS_SPAN);
    let targets = i32::from(rank) * if span { 2 } else { 1 };
    // SimC's recipient count is action-wide. TC741–781 shares the native
    // Dummy0 and damage1 selection: its leading Dummy must carry the cap too.
    for effect in &mut child.effects {
        effect.chain_targets = targets;
    }
    child.cast_empower_level = Some(rank);
    child
}

pub(super) fn release(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    let rank = spell
        .cast_empower_level
        .expect("empower pipeline supplies its completed stage");
    let child = match spell.id {
        357208 | 382266 => build_fire_breath_release(world, caster, rank),
        359073 | 382411 => build_eternity_surge_release(world, caster, rank),
        _ => unreachable!("qualified empower handler registration"),
    };
    // TC uses IGNORE_CAST_IN_PROGRESS, not FULL_MASK: preserve normal damage procs.
    crate::spell_cast::execute_cast(world, caster, &child, target)?;
    extend_dragonrage(world, caster);
    Ok(())
}

pub(crate) fn damage_bonus(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    let Some(rank) = spell.cast_empower_level.filter(|_| spell.id == FIRE_DAMAGE) else {
        return 0.0;
    };
    let periodic = spell
        .effects
        .iter()
        .find(|effect| effect.index == 1 && effect.aura == 3)
        .expect("native Fire Breath periodic effect1");
    let tick = crate::spell_cast::effects::scaled_amount(world, caster, spell, periodic);
    tick * (u32::from(rank - 1) * CONVERTED_TICKS_PER_RANK) as f32
}
