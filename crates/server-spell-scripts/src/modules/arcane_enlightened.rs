//! Enlightened samples real Mana on its native periodic clock.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraInstance, AuraSource};
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;
use shared::components::{ActiveSpec, Mana};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::TargetDamageMultiplier(damage_multiplier),
];

const TALENT: u32 = 321387;
const ENLIGHTENED: u32 = 1217242;
const ARCANE_SCHOOL: u32 = 64;

#[derive(Component)]
struct EnlightenedMana(f32);

fn register(scripts: &mut auras::scripts::AuraScripts) {
    scripts.0.entry(TALENT).or_default().on_apply = Some(activate);
    scripts.0.entry(TALENT).or_default().on_remove = Some(deactivate);
    scripts.0.entry(ENLIGHTENED).or_default().on_apply = Some(sample);
    scripts.0.entry(ENLIGHTENED).or_default().on_periodic = Some(sample_tick);
    scripts.0.entry(ENLIGHTENED).or_default().on_remove = Some(remove_sample);
}

fn activate(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit
        || world
            .get::<ActiveSpec>(unit)
            .is_none_or(|spec| spec.0 != 62)
    {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: ENLIGHTENED,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn deactivate(world: &mut World, unit: Entity, aura: &AuraInstance, _: auras::RemoveReason) {
    if aura.caster == unit {
        auras::remove_auras(world, unit, auras::RemoveReason::Consumed, |active| {
            active.caster == unit && active.spell_id == ENLIGHTENED
        });
    }
}

fn sample(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit {
        return;
    }
    let mana = world.get::<Mana>(unit).expect("Enlightened caster Mana");
    let fraction = mana.current / mana.max;
    world.entity_mut(unit).insert(EnlightenedMana(fraction));
}

fn sample_tick(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    if effect == 0 {
        sample(world, unit, aura);
    }
}

fn remove_sample(world: &mut World, unit: Entity, aura: &AuraInstance, _: auras::RemoveReason) {
    if aura.caster == unit {
        world.entity_mut(unit).remove::<EnlightenedMana>();
    }
}

fn damage_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if spell.school_mask() & ARCANE_SCHOOL == 0 {
        return 1.0;
    }
    let Some(sample) = world.get::<EnlightenedMana>(caster) else {
        return 1.0;
    };
    // sc_mage.cpp6392–6399/6793–6798: source buff samples Mana every native second.
    1.0 + sample.0 * script_effect_value(world, caster, ENLIGHTENED, 1) / 100.0
}
