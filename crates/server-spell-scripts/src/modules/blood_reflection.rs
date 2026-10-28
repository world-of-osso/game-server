//! Bloody Reflection damages the attacker while Blood Shield survives a direct hit.
use bevy::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

use super::combat;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::HealthChangeObserved(reflect)];

#[derive(Component)]
struct ReflectionCooldown(f64);

// SimC76f8118 sc_death_knight.cpp15540/16675–16682: surviving Blood Shield
// and a 500ms internal cooldown. Spelltext1279633 requires direct damage.
fn reflect(world: &mut World, event: &CombatLogEvent) {
    if !matches!(
        event.kind,
        CombatLogKind::Damage | CombatLogKind::Miss(MissKind::Absorb)
    ) || event.periodic
        || event.amount + event.absorbed <= 0
    {
        return;
    }
    let (Some(attacker), Some(target)) = (
        event.source.and_then(Entity::try_from_bits),
        event.target.and_then(Entity::try_from_bits),
    ) else {
        return;
    };
    if attacker == target
        || !combat::auras::has_aura(world, target, 1279633)
        || !combat::auras::has_aura(world, target, 77535)
    {
        return;
    }
    let now = spell_cast::now(world);
    if world
        .get::<ReflectionCooldown>(target)
        .is_some_and(|cooldown| now < cooldown.0)
    {
        return;
    }
    world
        .entity_mut(target)
        .insert(ReflectionCooldown(now + 0.5));
    spell_cast::cast_triggered(world, target, 1279656, Some(attacker), 1);
}
