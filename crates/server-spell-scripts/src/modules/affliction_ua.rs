//! Native Unstable Affliction stack-expiration producers.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance, scripts::AuraScripts};
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const UNSTABLE_AFFLICTION: u32 = 1259790;
const FATAL_ECHOES: u32 = 1260229;
const CASCADING_CALAMITY: u32 = 1261124;
const CALAMITY_BUFF: u32 = 1261125;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::ConsumedEffectValues(&[(FATAL_ECHOES, 0)]),
];

#[derive(Component)]
struct EchoAttempts {
    passive: u32,
    count: u32,
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(UNSTABLE_AFFLICTION).or_default().on_stack_loss = Some(expire_stacks);
}

// SimC76f8118 sc_warlock_init.cpp1248–1253: one native-chance PRD stream
// per caster, shared by all of that caster's actual expired UA applications.
fn roll_echo(world: &mut World, caster: Entity) -> bool {
    let Some(passive) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| aura.caster == caster && aura.spell_id == FATAL_ECHOES)
            .map(|aura| aura.instance_id)
    }) else {
        return false;
    };
    let chance = combat::spell_cast::scripts::script_effect_value(world, caster, FATAL_ECHOES, 0);
    let constant = combat::random::accumulated_proc_constant(f64::from(chance) / 100.0);
    if world.get::<EchoAttempts>(caster).is_none_or(|state| state.passive != passive) {
        world.entity_mut(caster).insert(EchoAttempts { passive, count: 0 });
    }
    let mut attempts = world.get_mut::<EchoAttempts>(caster).unwrap();
    attempts.count = attempts.count.saturating_add(1);
    let success = combat::random::random::<f64>() < constant * f64::from(attempts.count);
    if success { attempts.count = 0; }
    success
}

fn grant_calamity(world: &mut World, caster: Entity) {
    if combat::auras::has_aura(world, caster, CASCADING_CALAMITY) {
        combat::spell_cast::cast_triggered(world, caster, CALAMITY_BUFF, Some(caster), 1);
    }
}

// SimC76f8118 sc_warlock_actions.cpp1587–1606/5757–5780: each actual
// expired application rolls once, including the final stack. A successful
// echo reapplies a full native UA for free, without spending Instability.
fn expire_stacks(world: &mut World, target: Entity, aura: &AuraInstance, lost: u8) {
    if !combat::spell_cast::targets::is_alive(world, target) {
        return;
    }
    for _ in 0..lost {
        if roll_echo(world, aura.caster) {
            schedule_echo(world, target, aura);
        }
    }
}

fn schedule_echo(world: &mut World, target: Entity, aura: &AuraInstance) {
    let spell = world.resource::<SpellStore>().get(UNSTABLE_AFFLICTION)
        .expect("expiring native UA remains in the spell store").clone();
    let ctx = EffectCtx {
        caster: aura.caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    // The final native aura is still stored during stack-loss dispatch. Delay
    // until its removal completes, as the pinned final-tick callback does.
    combat::spell_cast::queued_triggers::schedule_full_trigger(
        world, &ctx, target, UNSTABLE_AFFLICTION, 1,
    );
    // Fatal Echoes also earns this buff when no previous UA remains.
    grant_calamity(world, aura.caster);
}
