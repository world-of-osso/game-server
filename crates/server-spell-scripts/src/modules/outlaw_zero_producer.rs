//! Zero In is earned only by critical white swings, with native dagger suppression.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::melee::Hand;
use combat::random::Rng;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use shared::components::WeaponType;

const TALENT: u32 = 1259485;
const ZERO_IN: u32 = 1259486;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::AfterWhiteHandHit(after_white_hit),
];

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

fn after_white_hit(world: &mut World, caster: Entity, hand: Hand, critical: bool) {
    if !critical || !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let stats = world.get::<combat::stats::CharacterStats>(caster).expect("native white swing has weapon stats");
    let weapon = match hand {
        Hand::Main => &stats.main_hand,
        Hand::Off => stats.off_hand.as_ref().expect("native off-hand swing has a weapon"),
    };
    // SimC76f8118 sc_rogue.cpp3422–3430: critical dagger swings are
    // suppressed by the talent's native effect2 percentage.
    if weapon.kind == WeaponType::Dagger {
        let chance = spell_cast::scripts::script_effect_value(world, caster, TALENT, 1);
        if combat::random::thread_rng().gen_range(0.0..100.0) < chance {
            return;
        }
    }
    spell_cast::cast_triggered(world, caster, ZERO_IN, Some(caster), 1);
}
