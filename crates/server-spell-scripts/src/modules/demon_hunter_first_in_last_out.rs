//! DB2 70448 First In, Last Out grants a max-health shield on the authored mobility casts.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::Health;

const TALENT: u32 = 1266497;
const SHIELD: u32 = 1266619;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TALENT),
    SpellScriptHook::AfterNativeLaunch(grant_mobility_shield),
];

fn grant_mobility_shield(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // Infernal Strike, Fel Rush and Shift are the three actions named by the native tooltip.
    if !matches!(spell.id, 189110 | 195072 | 1234796)
        || !combat::auras::has_aura(world, caster, TALENT)
    {
        return;
    }
    let maximum = world
        .get::<Health>(caster)
        .expect("living mobility caster")
        .max;
    let percent = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    let mut shield = spell_cast::load_triggered_spell(world, SHIELD, 1)
        .expect("selected First In, Last Out requires its native shield");
    shield
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native First In, Last Out absorb")
        .base_points = maximum * percent / 100.0;
    assert!(
        spell_cast::launch_triggered_spell(world, caster, &shield, Some(caster), 1),
        "First In, Last Out shield refused for {caster:?}"
    );
}
