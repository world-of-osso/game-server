//! SimC76f8118 sc_monk.cpp1805–1843/3970–3998; native spell payloads.
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::class_progression::TraitLoadout;
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

pub(super) const FOF: u32 = 113656;
pub(super) const ZENITH: u32 = 1249625;
pub(super) const JADEFIRE: u32 = 1248815;
pub(super) const RWK_DAMAGE: u32 = 468179;
const FOF_DAMAGE: u32 = 117418;
const MID2_TWO_PIECE: u32 = 1296621;
const JADEFIRE_TALENT: u32 = 457974;
const PATH_OF_JADE: u32 = 392994;
const RWK: u32 = 467307;

pub(super) fn effect_enabled(spell: u32, index: u8) -> bool {
    // SimC1805: FoF's parent has attack_power_mod.direct=0; children own damage.
    !(spell == FOF && index == 4)
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == talent)
    })
}

pub(super) fn fof_executed(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC1805–1827: the real equipped two-piece adds one half-strength child.
    let percent = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == MID2_TWO_PIECE && aura.caster == caster)
            .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
            .map(|effect| effect.amount)
    });
    if let Some(percent) = percent {
        cast_mid2_tick(world, caster, spell, percent);
    }
    super::windwalker_flurry::release(world, caster, spell);
}

fn cast_mid2_tick(world: &mut World, caster: Entity, parent: &SpellInfo, percent: f32) {
    let Some(primary) = parent.cast_primary_target else {
        warn!(?caster, "FoF tier child lacks its cast primary");
        return;
    };
    let mut child = world
        .resource::<SpellStore>()
        .get(FOF_DAMAGE)
        .expect("native FoF damage child")
        .clone();
    child.cast_damage_multiplier = Some(percent / 100.0);
    if !super::super::launch_triggered_spell(world, caster, &child, Some(primary), 1) {
        warn!(?caster, "Native FoF tier child refused");
    }
}

pub(super) fn channel_ended(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    // SimC1829–1843 and dot.cpp75–81 also run last_tick on interruption.
    if aura.spell_id != FOF
        || unit != aura.caster
        || !matches!(reason, RemoveReason::ChannelEnded | RemoveReason::Expired)
        || !selected(world, unit, JADEFIRE_TALENT)
    {
        return;
    }
    let cap = super::script_effect_value(world, unit, JADEFIRE_TALENT, 1) as u32;
    let mut child = world
        .resource::<SpellStore>()
        .get(JADEFIRE)
        .expect("native Jadefire damage child")
        .clone();
    child
        .target_restrictions
        .get_or_insert_with(Default::default)
        .max_targets = cap;
    if !super::super::launch_triggered_spell(world, unit, &child, Some(unit), 1) {
        warn!(?unit, "Native Jadefire child refused");
    }
}

pub(super) fn zenith_hit(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    // SimC3993–3995 resets the real RSK recovery, including shared charges.
    super::windwalker::reset_rising_sun_kick(world, caster);
}

fn target_count(world: &World, caster: Entity, spell: u32) -> f32 {
    let frame = world
        .resource::<super::super::ResolvingHits>()
        .0
        .last()
        .expect("damage modifier runs inside its native hit frame");
    assert_eq!((frame.caster, frame.spell_id), (caster, spell));
    frame.selected.len() as f32
}

pub(super) fn jadefire_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    // SimC1761–1771: selected Path of Jade uses the actual selected count.
    if !selected(world, caster, PATH_OF_JADE) {
        return amount;
    }
    let percent = super::script_effect_value(world, caster, PATH_OF_JADE, 0);
    let cap = super::script_effect_value(world, caster, PATH_OF_JADE, 1);
    let count = target_count(world, caster, JADEFIRE).min(cap);
    amount * (1.0 + count * percent / 100.0)
}

pub(super) fn rushing_wind_damage(
    world: &World,
    caster: Entity,
    _target: Entity,
    amount: f32,
) -> f32 {
    // SimC1153–1174: split across actual targets after authored per-target scaling.
    let count = target_count(world, caster, RWK_DAMAGE);
    let percent = super::script_effect_value(world, caster, RWK, 0);
    let cap = super::script_effect_value(world, caster, RWK, 1);
    amount * (1.0 + count.min(cap) * percent / 100.0) / count
}
