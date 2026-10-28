//! SimC76f8118 sc_monk.cpp1385–1386, 2596 and 4395–4448: Shuffle duration.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use combat::spell_info::{SpellInfo, SpellStore};

const SHUFFLE_TALENT: u32 = 322120;
const SHUFFLE: u32 = 215479;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 205523 | 121253).then_some(extend_shuffle)
}

fn extend_shuffle(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == SHUFFLE_TALENT)
    }) {
        return;
    }
    let effect = if spell.id == 205523 { 1 } else { 5 };
    let added = (script_effect_value(world, caster, spell.id, effect) * 1000.0) as u32;
    let now = combat::spell_cast::now(world);
    let remaining = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SHUFFLE && aura.caster == caster)
                .and_then(|aura| aura.expires_at)
        })
        .map_or(0, |expiry| ((expiry - now).max(0.0) * 1000.0) as u32);
    let max_duration = {
        let info = world
            .resource::<SpellStore>()
            .get(SHUFFLE)
            .expect("native Shuffle metadata");
        combat::spell_cast::duration_ms(world, caster, info).max(0) as u32 * 3
    };
    if remaining == 0 {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: SHUFFLE,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    auras::set_owned_duration(
        world,
        caster,
        caster,
        SHUFFLE,
        remaining.saturating_add(added).min(max_duration),
    );
}
