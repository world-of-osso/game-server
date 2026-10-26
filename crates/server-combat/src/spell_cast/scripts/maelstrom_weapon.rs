//! SimC76f8118 sc_shaman.cpp2640–2675/10919–11023/11870–11902:
//! landed melee earns native stacks; foreground affected spells snapshot and spend them.
//! TC a352b1fa spell_shaman.cpp1973–1993/2056–2071 supplies the modifier/counter pair.
use bevy::prelude::*;

use crate::auras::procs::AuraEvent;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_cast;
use crate::spell_info::SpellInfo;

const PASSIVE: u32 = 187880;
const COUNTER: u32 = 344179;
pub(super) const MODIFIER: u32 = 187881;
const GHOST_WOLF: u32 = 2645;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        PASSIVE,
        AuraScript {
            check_proc: Some(check_melee),
            on_proc: Some(earn_melee_stack),
            ..Default::default()
        },
    );
    scripts.0.insert(
        384437,
        AuraScript {
            on_periodic: Some(generate_static_accumulation),
            ..Default::default()
        },
    );
    scripts.0.insert(
        COUNTER,
        AuraScript {
            on_remove: Some(remove_modifier),
            ..Default::default()
        },
    );
}

fn check_melee(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.damage > 0.0 && !event.periodic && !auras::has_aura(world, aura.caster, GHOST_WOLF)
}

fn earn_melee_stack(
    world: &mut World,
    caster: Entity,
    _aura: &AuraInstance,
    _target: Option<Entity>,
) {
    generate(world, caster, 1);
}

pub(super) fn generate(world: &mut World, caster: Entity, count: u8) {
    if !auras::has_aura(world, caster, PASSIVE) {
        return;
    }
    for _ in 0..count {
        // Amounts, caps and duration are calculated by the real native aura path.
        spell_cast::cast_triggered(world, caster, MODIFIER, Some(caster), 1);
        spell_cast::cast_triggered(world, caster, COUNTER, Some(caster), 1);
    }
}

pub(super) fn grant_voltaic_blaze(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC sc_shaman.cpp10123–10125: once on execute, not per hit or Maelstrom power.
    let count = super::script_effect_value(world, caster, spell.id, 1) as u8;
    generate(world, caster, count);
    super::enhancement_nova::roll_and_cast(world, caster);
    super::enhancement_tier::apply_burning_core(world, caster, spell);
}

fn remove_modifier(world: &mut World, caster: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    auras::remove_auras(world, caster, RemoveReason::Consumed, |child| {
        child.spell_id == MODIFIER && child.caster == aura.caster
    });
}

fn generate_static_accumulation(world: &mut World, caster: Entity, aura: &AuraInstance, index: u8) {
    // SimC76f8118 sc_shaman.cpp12838–12847: actual mode-owned ticks earn the selected talent's stacks.
    if index != 0 || !auras::has_aura(world, caster, 384411) {
        return;
    }
    let count = aura
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("native Static Accumulation tick amount")
        .amount as u8;
    generate(world, caster, count);
}

pub(crate) fn snapshots_modifier(id: u32) -> bool {
    id == MODIFIER
}

pub(crate) fn consume(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let admitted = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == MODIFIER && spell.cast_proc_mods.contains(&aura.instance_id)
        })
    });
    if !admitted {
        return;
    }
    // SimC2647–2649 uses the live modified driver's per-cast limit.
    let limit = super::script_effect_value(world, caster, PASSIVE, 1) as u8;
    let Some(counter) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == COUNTER && aura.caster == caster)
            .cloned()
    }) else {
        return;
    };
    let spent = counter.stacks.min(limit);
    let remaining = counter.stacks - spent;
    if remaining == 0 {
        auras::remove_instance(world, caster, counter.instance_id, RemoveReason::Consumed);
    } else {
        retain_stacks(world, caster, remaining);
    }
    reduce_tempo_recharge(world, caster, spent);
    super::enhancement_tempest::grant_for_spending(world, caster, spent);
}

fn reduce_tempo_recharge(world: &mut World, caster: Entity, spent: u8) {
    // SimC76f8118 sc_shaman.cpp11780–11787: each actual spent stack reduces both native clocks.
    const ELEMENTAL_TEMPO: u32 = 1250364;
    if spent == 0 || !auras::has_aura(world, caster, ELEMENTAL_TEMPO) {
        return;
    }
    let milliseconds = super::script_effect_value(world, caster, ELEMENTAL_TEMPO, 2);
    let seconds = f64::from(milliseconds) * f64::from(spent) / 1000.0;
    super::reduce_cooldown(world, caster, 17364, seconds);
    super::reduce_cooldown(world, caster, 60103, seconds);
}

fn retain_stacks(world: &mut World, caster: Entity, remaining: u8) {
    auras::set_self_cast_stacks(world, caster, COUNTER, remaining);
    auras::set_self_cast_stacks(world, caster, MODIFIER, remaining);
}
