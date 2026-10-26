//! SimC76f8118 sc_priest.cpp1683–1685/3540–3541/3571–3576.
//! Actual Rift earns native Voidheart; selected Touch retains its authored tail.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraInstance, AuraSource, RemoveReason};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const TALENT: u32 = 449880;
const TOUCH: u32 = 1266856;
const RIFT: u32 = 459314;
pub(super) const BUFF: u32 = 449887;
const BONUS_EFFECTS: u32 = 0b11;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(earn_bonus),
    SpellScriptHook::ConsumedEffectValues(&[(TALENT, 0), (TALENT, 1), (TOUCH, 0)]),
];

fn earn_bonus(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.spell_id != RIFT || aura.caster != unit {
        return;
    }
    if auras::has_aura(world, unit, TALENT) {
        apply_bonus(world, unit);
    }
}

fn apply_bonus(world: &mut World, caster: Entity) {
    // SimC buff.trigger applies the aura, not a helpful cast with proc side effects.
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: BUFF,
            effect_mask: BONUS_EFFECTS,
            source: AuraSource::Cast,
        },
    );
}

pub(super) fn replace_with_touch_tail(world: &mut World, caster: Entity) {
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == BUFF && aura.caster == caster
    });
    if !auras::has_aura(world, caster, TOUCH) {
        return;
    }
    let seconds = spell_cast::scripts::script_effect_value(world, caster, TOUCH, 0);
    apply_bonus(world, caster);
    auras::set_owned_duration(world, caster, caster, BUFF, (seconds * 1000.0) as u32);
}
