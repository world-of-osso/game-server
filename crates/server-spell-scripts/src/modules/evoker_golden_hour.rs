//! Golden Hour: recorded damage, including absorbs, in the preceding native window.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::VecDeque;

const TALENT: u32 = 378196;
const HEAL: u32 = 378213;
const WINDOW: f64 = 5.0;
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::HealthChangeObserved(record_damage),
    SpellScriptHook::AfterForegroundSpecial(heal_reversion_target),
];

#[derive(Component, Default)]
struct RecentDamage(VecDeque<(f64, f32)>);

fn record_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage {
        return;
    }
    let Some(target) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    if world.get::<shared::components::Player>(target).is_none()
        && world.get::<combat::followers::Follower>(target).is_none()
    {
        return;
    }
    let amount = event.amount.max(0) as f32 + event.absorbed.max(0) as f32;
    if amount == 0.0 {
        return;
    }
    let now = spell_cast::now(world);
    let mut entity = world.entity_mut(target);
    let mut history = entity.entry::<RecentDamage>().or_default();
    let mut history = history.get_mut();
    while history
        .0
        .front()
        .is_some_and(|(time, _)| *time < now - WINDOW)
    {
        history.0.pop_front();
    }
    history.0.push_back((now, amount));
}

pub(super) fn is_reversion(spell: u32) -> bool {
    // DB2 378196 effects have empty SpellClassMask fields; 1256581 overrides 366155.
    matches!(spell, 366155 | 1256581)
}

fn heal_reversion_target(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !is_reversion(spell.id) {
        return;
    }
    if let Some(target) = spell.cast_primary_target {
        heal_target(world, caster, target, 1.0);
    }
}

pub(super) fn heal_target(world: &mut World, caster: Entity, target: Entity, scale: f32) {
    if !combat::auras::has_aura(world, caster, TALENT) {
        return;
    }
    let now = spell_cast::now(world);
    let damage: f32 = world.get::<RecentDamage>(target).map_or(0.0, |history| {
        history
            .0
            .iter()
            .filter(|(time, _)| *time >= now - WINDOW)
            .map(|(_, amount)| amount)
            .sum()
    });
    if damage <= 0.0 {
        return;
    }
    let percent = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    let heal = world
        .resource::<SpellStore>()
        .get(HEAL)
        .expect("native Golden Hour heal")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &heal,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    // INFERRED_WCL vhKYp3J2BcVdQ7fa f2 p1: 15% of damage plus absorbs exactly matches the receipt.
    effects::apply_fixed_heal(world, &ctx, target, damage * percent / 100.0 * scale);
}
