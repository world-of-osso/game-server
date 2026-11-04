//! DB2 70448 Indomitable: actual paid Rage grants a proportional max-health heal.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, CostPool, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::{ActiveSpec, PowerType};

const TALENT: u32 = 202095;
const HEAL: u32 = 316643;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterSpellPayment(heal_paid_rage)];

fn heal_paid_rage(world: &mut World, caster: Entity, _: &SpellInfo, costs: &[(CostPool, f32)]) {
    let protection = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 73);
    if !protection || !combat::auras::has_aura(world, caster, TALENT) {
        return;
    }
    let paid = spell_cast::spent(costs, PowerType::Rage);
    if paid <= 0.0 {
        return;
    }
    // The tooltip divides both effects by ten: effect 1 is already in stored Rage units.
    let threshold = spell_cast::scripts::script_effect_value(world, caster, TALENT, 1);
    let heal_percent = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) / 10.0;
    assert!(
        threshold > 0.0,
        "Indomitable requires its native Rage divisor"
    );
    let mut heal = spell_cast::load_triggered_spell(world, HEAL, 1)
        .expect("selected Indomitable requires its native heal");
    heal.effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Indomitable percent-health heal")
        .base_points = heal_percent * paid / threshold;
    assert!(
        spell_cast::launch_triggered_spell(world, caster, &heal, Some(caster), 1),
        "Indomitable heal refused for {caster:?}"
    );
}
