//! SimC76f8118 sc_druid.cpp5076–5121 and 11350–11356.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::random::Rng;
use combat::spell_cast::{
    self, CostPool, effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook,
    summon::SummonedCreature, targets,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::PowerType;

const TALENT: u32 = 1269614;
const THIRD_TALENT: u32 = 1269619;
const SUMMON: u32 = 1308399;
const ANSWERED_CALLING: u32 = 1308647;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::AfterPayment(on_payment),
    SpellScriptHook::AfterResolvedHits(spirit_attack),
    SpellScriptHook::AfterForegroundSpecial(wild_guardian_action),
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TALENT),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if !matches!(spell.id, 33917 | 77758)
        || !combat::spell_cast::script_registry::reach_source_selected(world, caster, TALENT)
    {
        return Default::default();
    }
    let wrath = world
        .resource::<SpellStore>()
        .get(1308093)
        .expect("native Spirit's Wrath");
    let range = wrath
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .expect("native Spirit's Wrath range")
        .max_hostile;
    combat::spell_cast::script_registry::ScriptReach::caster(range)
}

fn talent_amount(world: &World, caster: Entity, spell: u32, index: u8) -> Option<f32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)?
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| effect.amount)
}

fn on_payment(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    if spell_cast::spent(costs, PowerType::Rage) <= 0.0 {
        return;
    }
    let Some(chance) = talent_amount(world, caster, TALENT, 1) else {
        return;
    };
    if active_spirit(world, caster) || combat::random::random::<f32>() >= chance / 100.0 {
        return;
    }
    summon_spirit(world, caster);
}

fn summon_spirit(world: &mut World, caster: Entity) {
    if active_spirit(world, caster) {
        return;
    }
    spell_cast::cast_triggered(world, caster, SUMMON, Some(caster), 1);
    if !active_spirit(world, caster) {
        return;
    }
    if talent_amount(world, caster, THIRD_TALENT, 0).is_some() {
        spell_cast::cast_triggered(world, caster, ANSWERED_CALLING, Some(caster), 1);
        for spell in [33917, 77758] {
            spell_cast::scripts::reduce_cooldown(world, caster, spell, f64::MAX);
        }
    }
}

// SimC grants the action override on Berserk/Incarnation and consumes it on use.
fn wild_guardian_action(world: &mut World, caster: Entity, spell: &SpellInfo) {
    match spell.id {
        102558 | 50334 if talent_amount(world, caster, THIRD_TALENT, 0).is_some() => {
            let override_aura = if spell.id == 102558 { 1269616 } else { 1270277 };
            spell_cast::cast_triggered(world, caster, override_aura, Some(caster), 1);
        }
        1269658 => {
            auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                aura.caster == caster && matches!(aura.spell_id, 1269616 | 1270277)
            });
        }
        6807 | 400254 => {
            let conduit = world.get::<ActiveAuras>(caster).and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 1308176 && aura.caster == caster)
                    .map(|aura| aura.instance_id)
            });
            if let Some(instance) = conduit {
                auras::spend_stack(world, caster, instance);
                summon_spirit(world, caster);
            }
        }
        _ => {}
    }
}

fn active_spirit(world: &mut World, caster: Entity) -> bool {
    let now = spell_cast::now(world);
    world
        .query::<&SummonedCreature>()
        .iter(world)
        .any(|summon| {
            summon.owner == caster
                && summon.spell_id == SUMMON
                && summon.expires_at.is_none_or(|end| end > now)
        })
}

// Mangle and Thrash execute one Spirit's Wrath at a random live enemy; its
// native energize is immediate and the damage lands after the source 200ms travel.
fn spirit_attack(world: &mut World, ctx: &EffectCtx<'_>) {
    let caster = ctx.caster;
    if !matches!(ctx.spell.id, 33917 | 77758) || !active_spirit(world, caster) {
        return;
    }
    let Some(origin) = targets::position(world, caster) else {
        return;
    };
    let range = world
        .resource::<SpellStore>()
        .get(1308093)
        .and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.range)
        .expect("Spirit's Wrath native range")
        .max_hostile;
    let candidates = targets::area_units(world, caster, origin, range, targets::Relation::Enemy, 0);
    if candidates.is_empty() {
        return;
    }
    let target = candidates[combat::random::thread_rng().gen_range(0..candidates.len())];
    if talent_amount(world, caster, THIRD_TALENT, 0).is_some() {
        spell_cast::cast_triggered(world, caster, 1308125, Some(caster), 1);
    }
    queued_triggers::schedule_full_trigger(world, ctx, target, 1308093, 200);
}
