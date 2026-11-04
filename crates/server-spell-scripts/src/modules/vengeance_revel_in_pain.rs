//! DB2 70448 Revel in Pain: Fire damage earns a capped shield, doubled during Fiery Brand.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use shared::protocol::{CombatLogEvent, CombatLogKind};

const TALENT: u32 = 343014;
const SHIELD: u32 = 1265857;
const FIERY_BRAND: u32 = 207771;
const FIRE_SCHOOL: u32 = 4;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::HealthChangeObserved(shield_fire_damage)];

fn shield_fire_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage
        || event.school_mask & FIRE_SCHOOL == 0
        || event.source == event.target
        || event.amount <= 0
    {
        return;
    }
    let Some(caster) = event.source.and_then(Entity::try_from_bits) else {
        return;
    };
    if !combat::auras::has_aura(world, caster, TALENT) {
        return;
    }
    let fraction = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) / 1000.0;
    let brand = if combat::auras::has_aura(world, caster, FIERY_BRAND) {
        2.0
    } else {
        1.0
    };
    let mut shield = spell_cast::load_triggered_spell(world, SHIELD, 1)
        .expect("selected Revel in Pain requires its native shield");
    shield
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Revel in Pain absorb")
        .base_points = event.amount as f32 * fraction * brand;
    // The source damage already contains outgoing modifiers, as for Ursoc's Fury.
    shield
        .misc
        .as_mut()
        .expect("native Revel in Pain metadata")
        .attributes[11] |= 0x0008_0000;
    assert!(
        spell_cast::launch_triggered_spell(world, caster, &shield, Some(caster), 1),
        "Revel in Pain shield refused for {caster:?}"
    );
}
