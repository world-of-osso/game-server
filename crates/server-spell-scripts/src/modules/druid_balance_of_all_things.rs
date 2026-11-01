//! SimC76f8118 sc_druid.cpp:3305–3339, 11080–11090: Eclipse starts reverse crit stacks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, scripts::AuraScripts};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const TALENT: u32 = 394048;
const SOLAR: u32 = 48517;
const SOLAR_BUFF: u32 = 394049;
const LUNAR_BUFF: u32 = 394050;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    for spell in [SOLAR_BUFF, LUNAR_BUFF] {
        scripts.0.entry(spell).or_default().on_periodic = Some(decay);
    }
}

pub(super) fn on_eclipse(world: &mut World, caster: Entity, eclipse: u32) {
    if !world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(TALENT))
    {
        return;
    }
    let buff = if eclipse == SOLAR {
        SOLAR_BUFF
    } else {
        LUNAR_BUFF
    };
    let stacks = world
        .resource::<SpellStore>()
        .get(buff)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Balance of All Things native maximum stacks")
        .cumulative_aura as u8;
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        buff,
        Some(caster),
        1,
        spell_cast::TriggeredSpellOverrides {
            aura_stacks: Some(stacks),
            ..Default::default()
        },
    );
}

fn decay(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if effect_index == 1 {
        auras::spend_stack(world, unit, aura.instance_id);
    }
}
