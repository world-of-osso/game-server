//! Insatiable Blade's Bone Shield stacks on Dancing Rune Weapon.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource};
use combat::class_progression::KnownSpells;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterForegroundSpecial(grant_bones)];

// SimC76f8118 sc_death_knight.cpp9500–9521 grants Insatiable Blade's
// authored effect2 stacks when Dancing Rune Weapon is cast.
fn grant_bones(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 49028
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(377637))
    {
        return;
    }
    let stacks = world
        .resource::<SpellStore>()
        .get(377637)
        .expect("Insatiable Blade talent")
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Insatiable Blade stacks")
        .base_points as u32;
    for _ in 0..stacks {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 195181,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}
