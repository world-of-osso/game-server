//! Blood Death Strike healing and its mastery absorb.
use std::collections::VecDeque;

use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    script_registry::{EnemyContextHitHook, SpellScriptHook},
};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::HealthChangeObserved(record_incoming_damage),
    SpellScriptHook::EnemyContextHit(death_strike_hit),
    SpellScriptHook::RawHeal(blood_shield_from_heal),
    SpellScriptHook::TargetContextDamageMultiplier(sanguination_damage),
];
const DEATH_STRIKE: u32 = 49998;
const HEAL: u32 = 45470;
const MASTERY: u32 = 77513;
const SHIELD: u32 = 77535;
const IMPROVED: u32 = 374277;
const VORACIOUS: u32 = 273953;
const VAMPIRIC_BLOOD: u32 = 55233;
const HEMOSTASIS: u32 = 273947;

// SimC76f8118 sc_death_knight.cpp10070–10091: Sanguination scales the
// owner's Death Strike with actual target missing health, capped at 80%.
fn sanguination_damage(world: &World, ctx: &EffectCtx<'_>, target: Entity, periodic: bool) -> f32 {
    if periodic
        || ctx.spell.id != DEATH_STRIKE
        || !blood(world, ctx.caster)
        || !combat::auras::has_aura(world, ctx.caster, 326801)
    {
        return 1.0;
    }
    let Some(health) = world
        .get::<Health>(target)
        .filter(|health| health.max > 0.0)
    else {
        return 1.0;
    };
    let bond = 1.0 + aura_effect(world, ctx.caster, 374261, 0).unwrap_or(0.0) / 100.0;
    let missing = (1.0 - health.current / health.max).clamp(0.0, 0.8);
    (1.0 + missing * (1.0 + 0.25 * bond)) * bond
}

#[derive(Component, Default)]
struct RecentDamage {
    hits: VecDeque<(f64, f32)>,
    last_consumed: f64,
}

fn blood(world: &World, unit: Entity) -> bool {
    world
        .get::<ActiveSpec>(unit)
        .is_some_and(|spec| spec.0 == 250)
}

fn aura_effect(world: &World, unit: Entity, spell: u32, index: u8) -> Option<f32> {
    world
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == unit)?
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| effect.amount)
}

fn base_effect(world: &World, spell: u32, index: u8) -> f32 {
    world
        .resource::<SpellStore>()
        .get(spell)
        .expect("native Blood spell")
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("native Blood effect")
        .base_points
}

// SimC76f8118 player.cpp8322–8331: record damage received, after mitigation.
fn record_incoming_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage {
        return;
    }
    let Some(unit) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    if !blood(world, unit) {
        return;
    }
    let amount = (event.amount - event.overflow.max(0)).max(0) as f32;
    if amount == 0.0 {
        return;
    }
    let now = spell_cast::now(world);
    let window = f64::from(base_effect(world, DEATH_STRIKE, 3));
    let mut entity = world.entity_mut(unit);
    let mut history = entity.entry::<RecentDamage>().or_default();
    let mut history = history.get_mut();
    while history
        .hits
        .front()
        .is_some_and(|(at, _)| *at < now - window)
    {
        history.hits.pop_front();
    }
    history.hits.push_back((now, amount));
}

#[derive(Component)]
struct PendingDeathStrikeHeal;

fn death_strike_hit(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == DEATH_STRIKE).then_some(schedule_death_strike_heal)
}

fn schedule_death_strike_heal(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if !blood(world, ctx.caster) || ctx.spell.cast_primary_target != Some(target) {
        return;
    }
    if ctx.depth == 0 {
        world.entity_mut(ctx.caster).insert(PendingDeathStrikeHeal);
    } else {
        heal_death_strike(world, ctx);
    }
}

pub(super) fn finish_death_strike_heal(world: &mut World, caster: Entity, spell: &combat::spell_info::SpellInfo) {
    if spell.id == DEATH_STRIKE && world.entity_mut(caster).take::<PendingDeathStrikeHeal>().is_some() {
        let ctx = EffectCtx { caster, spell, depth: 0, dest: None, chain_multiplier: 1.0 };
        heal_death_strike(world, &ctx);
    }
}

// SimC76f8118 sc_death_knight.cpp9945–10024: the larger of the native
// minimum-health and recent-damage amounts, with the consumed-window rule.
fn heal_death_strike(world: &mut World, ctx: &EffectCtx<'_>) {
    let caster = ctx.caster;
    let now = spell_cast::now(world);
    let window = f64::from(base_effect(world, DEATH_STRIKE, 3));
    let max_health = world
        .get::<Health>(caster)
        .expect("Death Strike caster health")
        .max;
    let improved = 1.0 + aura_effect(world, caster, IMPROVED, 3).unwrap_or(0.0) / 100.0;
    let min_pct = base_effect(world, DEATH_STRIKE, 2)
        * (1.0 + aura_effect(world, caster, VORACIOUS, 2).unwrap_or(0.0) / 100.0)
        * improved
        / 100.0;
    let damage_pct = base_effect(world, DEATH_STRIKE, 1)
        * (1.0 + aura_effect(world, caster, VORACIOUS, 1).unwrap_or(0.0) / 100.0)
        * improved
        / 100.0;
    let minimum = max_health * min_pct;
    let (recent, full_window) = world
        .get::<RecentDamage>(caster)
        .map_or((0.0, 0.0), |history| {
            let sum = |cutoff: f64| {
                history
                    .hits
                    .iter()
                    .filter(|(at, _)| *at >= cutoff)
                    .map(|(_, value)| *value)
                    .sum::<f32>()
            };
            (
                sum((now - window).max(history.last_consumed)),
                sum(now - window),
            )
        });
    if full_window * damage_pct > minimum
        && let Some(mut history) = world.get_mut::<RecentDamage>(caster)
    {
        history.last_consumed = now;
    }
    let spell = world
        .resource::<SpellStore>()
        .get(HEAL)
        .expect("native Death Strike heal")
        .clone();
    let heal_ctx = EffectCtx {
        caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::apply_heal(
        world,
        &heal_ctx,
        caster,
        minimum.max(recent * damage_pct),
        false,
        false,
    );
    // SimC76f8118 sc_death_knight.cpp10006–10012: the heal earns Voracious leech.
    if combat::auras::has_aura(world, caster, VORACIOUS) {
        spell_cast::cast_triggered(world, caster, 274009, Some(caster), ctx.depth + 1);
    }
}

// SimC76f8118 sc_death_knight.cpp10125–10130 consumes the whole buff
// after the Death Strike heal has used its stacks, including on a miss.
pub(super) fn consume_hemostasis(world: &mut World, caster: Entity, spell: &combat::spell_info::SpellInfo) {
    if spell.id == DEATH_STRIKE && blood(world, caster) {
        combat::auras::remove_auras(
            world,
            caster,
            combat::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == HEMOSTASIS && aura.caster == caster,
        );
    }
}

// SimC76f8118 sc_death_knight.cpp6952–6983: raw healing, mastery and
// no Vampiric Blood healing bonus; the shield refresh carries the unpaid bank.
fn blood_shield_from_heal(world: &mut World, ctx: &EffectCtx<'_>, raw: f32) {
    if ctx.spell.id != HEAL || !blood(world, ctx.caster) {
        return;
    }
    let caster = ctx.caster;
    let Some(mastery) = aura_effect(world, caster, MASTERY, 0) else {
        return;
    };
    let vamp = 1.0 + aura_effect(world, caster, VAMPIRIC_BLOOD, 0).unwrap_or(0.0) / 100.0;
    let Some(mut shield) = spell_cast::load_triggered_spell(world, SHIELD, ctx.depth + 1) else {
        return;
    };
    shield
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Blood Shield absorb")
        .base_points = raw * mastery / 100.0 / vamp;
    // SimC absorb_t snapshots no player/target multipliers, including Vampiric Blood's absorb bonus.
    shield.misc.as_mut().expect("Blood Shield misc").attributes[11] |= 0x0008_0000;
    spell_cast::launch_triggered_spell(world, caster, &shield, Some(caster), ctx.depth + 1);
}
