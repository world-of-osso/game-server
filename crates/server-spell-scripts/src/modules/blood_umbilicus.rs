//! Umbilicus Eternus accumulates owned Blood Plague during Vampiric Blood.
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::combat;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const TALENT: u32 = 391517;
const ACCUMULATOR: u32 = 391519;
const SHIELD: u32 = 391527;
const VAMPIRIC_BLOOD: u32 = 55233;
const BLOOD_PLAGUE: u32 = 55078;

#[derive(Component)]
struct PendingUmbilicus(f32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterAuraApplication(start_accumulator),
    SpellScriptHook::HealthChangeObserved(accumulate_damage),
];

fn register(scripts: &mut AuraScripts) {
    // The scripted Vampiric Blood boundary owns the proc, not arbitrary damage.
    scripts.0.entry(TALENT).or_default().check_proc = Some(|_, _, _| false);
    scripts.0.entry(ACCUMULATOR).or_default().on_remove = Some(finish_accumulator);
}

fn start_accumulator(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id != VAMPIRIC_BLOOD
        || aura.caster != target
        || !auras::has_aura(world, target, TALENT)
    {
        return;
    }
    if !spell_cast::try_cast_triggered(world, target, ACCUMULATOR, Some(target), 1) {
        warn!(?target, "Umbilicus accumulator cast refused");
        return;
    }
    assert!(auras::set_owned_duration(
        world,
        target,
        target,
        ACCUMULATOR,
        aura.duration_ms
    ));
}

fn accumulate_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage
        || event.spell_id != Some(BLOOD_PLAGUE)
        || !event.periodic
    {
        return;
    }
    let Some(caster) = event.source.and_then(Entity::try_from_bits) else {
        return;
    };
    let Some((instance, previous)) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == ACCUMULATOR && aura.caster == caster)
            .and_then(|aura| {
                aura.effects
                    .iter()
                    .find(|effect| effect.index == 0)
                    .map(|effect| (aura.instance_id, effect.amount))
            })
    }) else {
        return;
    };
    let amount = (event.amount + event.absorbed).max(0) as f32;
    assert!(auras::set_effect_amount(
        world,
        caster,
        instance,
        0,
        previous + amount
    ));
}

fn finish_accumulator(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    if reason != RemoveReason::Expired || aura.caster != target {
        return;
    }
    let damage = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Umbilicus accumulator effect")
        .amount;
    if damage <= 0.0 {
        return;
    }
    let factor = spell_cast::scripts::script_effect_value(world, target, TALENT, 0);
    world
        .entity_mut(target)
        .insert(PendingUmbilicus(damage * factor));
    let spell = world
        .resource::<SpellStore>()
        .get(TALENT)
        .expect("Umbilicus talent")
        .clone();
    let ctx = EffectCtx {
        caster: target,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    // WCL observes the shield after Vampiric Blood's removal.
    spell_cast::queued_triggers::schedule_callback(world, &ctx, target, 25, apply_shield);
}

fn apply_shield(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let amount = world
        .entity_mut(target)
        .take::<PendingUmbilicus>()
        .expect("pending Umbilicus shield")
        .0;
    let max_health = world
        .get::<Health>(target)
        .expect("Umbilicus target health")
        .max;
    // INFERRED_WCL: JpHdnbhYD8ZFWvma/1, z1C2HFkf6VpGqWZD/1,
    // MzD8pFb9YaW6fA1K/15, LnMDdV3NPCK9J8jG/4. Post-expiry cap and
    // absorb modifiers fit at median 0.948, p10–p90 0.890–1.124; 24/149
    // shields were within 5%. Retire when retail decompilation or server
    // evidence resolves the formula.
    if !spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        SHIELD,
        Some(target),
        depth,
        spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(amount.min(max_health)),
            ..Default::default()
        },
    ) {
        warn!(?target, "Umbilicus shield cast refused");
    }
}
