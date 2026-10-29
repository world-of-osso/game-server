//! SimC76f8118 sc_monk.cpp3070–3108 and 4155–4167: Brew casts launch Special Delivery.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::{SpellInfo, SpellStore};

const TALENT: u32 = 196730;
const MISSILE: u32 = 196732;
const DAMAGE: u32 = 196733;
const CELESTIAL_FLAMES: u32 = 1263641;
const FLAMES_DAMAGE: u32 = 1263667;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 115399 | 119582 | 115203 | 120954 | 322507 | 1241059).then_some(launch)
}

fn launch(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 268)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
    {
        return;
    }
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0)
    else {
        return;
    };
    let Some(position) = spell_cast::targets::position(world, target) else {
        return;
    };
    let delay = world
        .resource::<SpellStore>()
        .get(MISSILE)
        .and_then(|spell| spell.misc.as_ref())
        .expect("native Special Delivery missile")
        .speed;
    let Some(mut spell) = spell_cast::load_triggered_spell(world, DAMAGE, 1) else {
        return;
    };
    spell
        .misc
        .as_mut()
        .expect("native Special Delivery damage misc")
        .launch_delay = delay;
    spell.cast_destination = Some(position.to_array());
    spell_cast::launch_triggered_spell(world, caster, &spell, None, 1);
    if auras::has_aura(world, caster, CELESTIAL_FLAMES) {
        spell_cast::cast_triggered(world, caster, FLAMES_DAMAGE, Some(target), 1);
    }
}
