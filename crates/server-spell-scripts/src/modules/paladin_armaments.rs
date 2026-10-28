//! Holy Armaments alternate through the native action-bar override aura.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraApplication, AuraSource, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::SuccessfulCastRelationships(after_cast)];

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // Native70448 spell432478 effect0 is aura332: misc432459 -> amount432472.
    // SimC76f8118 sc_paladin.cpp2748–2752 alternates only on actual hardcasts.
    let Some(override_spell) = world.resource::<SpellStore>().get(432478) else {
        if matches!(spell.id, 432459 | 432472) {
            warn!("Holy Armaments override aura432478 is missing");
        }
        return;
    };
    let Some(effect) = override_spell
        .effects
        .iter()
        .find(|effect| effect.aura == 332)
    else {
        if matches!(spell.id, 432459 | 432472) {
            warn!("Holy Armaments override aura432478 lacks aura332");
        }
        return;
    };
    let override_id = override_spell.id;
    let base = effect.misc_values[0] as u32;
    let replacement = effect.base_points as u32;
    if spell.id == base {
        combat::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: override_id,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    } else if spell.id == replacement {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == override_id && aura.caster == caster
        });
    }
}
