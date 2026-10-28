//! SimC76f8118 sc_shaman.cpp8845–8856/11600–11613: Doom grants wolf buffs.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraInstance, AuraSource};
use combat::class_progression::KnownSpells;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

const SURGE: u32 = 224127;
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AsynchronousAuras(&[SURGE]),
    SpellScriptHook::AfterAuraApplication(after_application),
];

fn after_application(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.spell_id != 466772
        || aura.caster != unit
        || !world
            .get::<ActiveSpec>(unit)
            .is_some_and(|spec| spec.0 == 263)
    {
        return;
    }
    let Some(known) = world.get::<KnownSpells>(unit) else {
        return;
    };
    let feral = known.contains(469314);
    let rolling = known.contains(454026);
    if feral {
        let store = world.resource::<SpellStore>();
        let duration = store
            .get(469329)
            .and_then(|spell| spell.misc.as_ref())
            .and_then(|misc| misc.duration)
            .expect("native Feral Spirit duration")
            .duration_ms;
        let count = combat::spell_cast::scripts::script_effect_value(world, unit, 469314, 0) as u32;
        for _ in 0..count {
            grant(world, unit, duration);
        }
    }
    if rolling {
        let duration =
            combat::spell_cast::scripts::script_effect_value(world, unit, 454026, 1) as i32;
        grant(world, unit, duration);
    }
}

fn grant(world: &mut World, caster: Entity, duration_ms: i32) {
    let mut spell = world
        .resource::<SpellStore>()
        .get(SURGE)
        .expect("native Crackling Surge")
        .clone();
    let duration = spell
        .misc
        .as_mut()
        .and_then(|misc| misc.duration.as_mut())
        .expect("native Crackling Surge duration");
    duration.duration_ms = duration_ms;
    duration.max_duration_ms = duration_ms;
    auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: SURGE,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
        &spell,
    );
}
