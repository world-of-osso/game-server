//! Full effective Spree points affect hand damage, not the capped native duration.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const SPREE: u32 = 51690;

#[derive(Component)]
struct SpreePower(f32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceOnlyContext(&[SPREE]),
    SpellScriptHook::BeforeNativeOnCast(capture_power),
];

fn capture_power(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if spell.id != SPREE {
        return;
    }
    clear(world, caster);
    // Pinned Spree is an ordinary paid action. A full-mask triggered root has
    // no paid/effective point context and cannot earn an excess-point bonus.
    if depth != 0 {
        return;
    }
    let points = spell
        .cast_policy_context
        .as_ref()
        .expect("native Spree retains its prepayment resource context")
        .resources
        .iter()
        .find_map(|(power, amount)| (*power == PowerType::ComboPoints).then_some(*amount))
        .expect("native Spree retains its effective combo-point value");
    let maximum = spell_cast::powers::power_value(world, caster, PowerType::ComboPoints)
        .expect("native Spree caster has a combo-point pool")
        .1;
    let excess = (points - maximum).max(0.0);
    if excess > 0.0 {
        // SimC76f8118 sc_rogue.cpp4702–4714:15% (native effect4) per
        // excess effective point.4794–4805 caps duration, not this damage value.
        let per_point = script_effect_value(world, caster, SPREE, 3) / 100.0;
        world
            .entity_mut(caster)
            .insert(SpreePower(1.0 + excess * per_point));
    }
}

pub(super) fn launch_hand(world: &mut World, caster: Entity, hand: u32, target: Entity) {
    let multiplier = world.get::<SpreePower>(caster).map_or(1.0, |power| power.0);
    let mut spell =
        spell_cast::load_triggered_spell(world, hand, 1).expect("native Spree hand payload");
    spell.cast_damage_multiplier = Some(multiplier);
    spell_cast::launch_triggered_spell(world, caster, &spell, Some(target), 1);
}

pub(super) fn clear(world: &mut World, caster: Entity) {
    world.entity_mut(caster).remove::<SpreePower>();
}
