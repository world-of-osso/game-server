//! Start the Demonology Diabolic Ritual from actual Soul Shard payments.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::powers::PowerTypeTable;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::{self, CostPool};
use combat::spell_info::SpellInfo;
use shared::components::{ActiveSpec, CombatStatus, PowerType};

const RITUAL: u32 = 428514;
const OVERLORD_RITUAL: u32 = 431944;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterSpellPayment(advance_ritual)];

fn advance_ritual(world: &mut World, caster: Entity, spell: &SpellInfo, costs: &[(CostPool, f32)]) {
    // SimC76f8118 actions.cpp278–355: paid Demonology Hand reduces the
    // native first ritual by effect1 seconds per Soul Shard spent.
    if spell.id != 105174 || world.get::<ActiveSpec>(caster).is_none_or(|spec| spec.0 != 266)
        || !auras::has_aura(world, caster, RITUAL)
        || world.get::<CombatStatus>(caster).is_none_or(|status| !status.0) {
        return;
    }
    let paid = spell_cast::spent(costs, PowerType::SoulShards);
    if paid <= 0.0 { return; }
    let units = world.resource::<PowerTypeTable>().get(PowerType::SoulShards)
        .expect("native Soul Shards display units").display_modifier;
    assert!(units > 0, "Soul Shards display divisor must be positive");
    let seconds = f64::from(spell_cast::scripts::script_effect_value(world, caster, RITUAL, 0))
        * f64::from(paid) / f64::from(units);
    if !auras::has_aura(world, caster, OVERLORD_RITUAL)
        && !spell_cast::try_cast_triggered(world, caster, OVERLORD_RITUAL, Some(caster), 1) {
        warn!(?caster, "Native Overlord ritual refused");
        return;
    }
    let now = spell_cast::now(world);
    let expiry = world.get::<ActiveAuras>(caster).and_then(|auras| auras.iter()
        .find(|aura| aura.spell_id == OVERLORD_RITUAL && aura.caster == caster))
        .and_then(|aura| aura.expires_at).expect("native ritual needs a finite expiry");
    let remaining_ms = ((expiry - now - seconds).max(0.0) * 1000.0).round() as u32;
    assert!(auras::set_owned_duration(world, caster, caster, OVERLORD_RITUAL, remaining_ms));
}
