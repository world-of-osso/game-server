//! SimC76f8118 sc_shaman.cpp4519–4532/6110–6135: real Crash executions own delayed Unleashed strikes.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, targets};
use combat::spell_info::SpellStore;

const CRASH: u32 = 187874;
const UNLEASHED: u32 = 1252373;
const DAMAGE: u32 = 1252431;
const SHORT_CIRCUIT: u32 = 1299991;
const PERIOD_SECONDS: f64 = 1.0;
// SimC1490: this is the pinned encounter occupancy model, not a damage coefficient.
const OCCUPANCY: f32 = 0.9;

struct Pulse {
    primary: Entity,
    multiplier: f32,
    next_tick: f64,
    remaining: u32,
    depth: u8,
}

#[derive(Component, Default)]
struct Pending(Vec<Pulse>);

pub(super) fn schedule_for_landed_root(world: &mut World, ctx: &EffectCtx, target: Entity) {
    let selected = world
        .get::<KnownSpells>(ctx.caster)
        .is_some_and(|known| known.contains(UNLEASHED));
    if ctx.spell.id != CRASH || !selected {
        return;
    }
    let pulse = Pulse {
        primary: target,
        multiplier: consume_short_circuit(world, ctx.caster),
        next_tick: auras::aura_now(world) + PERIOD_SECONDS,
        remaining: super::script_effect_value(world, ctx.caster, UNLEASHED, 1) as u32,
        depth: ctx.depth + 1,
    };
    if let Some(mut pending) = world.get_mut::<Pending>(ctx.caster) {
        pending.0.push(pulse);
    } else {
        world.entity_mut(ctx.caster).insert(Pending(vec![pulse]));
    }
}

fn consume_short_circuit(world: &mut World, caster: Entity) -> f32 {
    let aura = world.get::<ActiveAuras>(caster).and_then(|active| {
        active
            .iter()
            .find(|aura| aura.spell_id == SHORT_CIRCUIT && aura.caster == caster)
            .cloned()
    });
    let Some(aura) = aura else {
        return 1.0;
    };
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Short Circuit snapshot effect0")
        .amount;
    let multiplier = 1.0 + amount * f32::from(aura.stacks) / 100.0;
    // SimC6111/6134–6135 snapshots this execution's value before consuming the actual buff.
    auras::remove_instance(world, caster, aura.instance_id, RemoveReason::Consumed);
    multiplier
}

pub(super) fn update(world: &mut World) {
    let now = auras::aura_now(world);
    let mut due = Vec::new();
    for (caster, mut pending) in world.query::<(Entity, &mut Pending)>().iter_mut(world) {
        pending.0.retain_mut(|pulse| {
            if now >= pulse.next_tick && pulse.remaining > 0 {
                due.push((caster, pulse.primary, pulse.multiplier, pulse.depth));
                pulse.remaining -= 1;
                pulse.next_tick += PERIOD_SECONDS;
            }
            pulse.remaining > 0
        });
    }
    for (caster, primary, multiplier, depth) in due {
        cast_pack_pulse(world, caster, primary, multiplier, depth);
    }
}

fn select_current_pack(world: &mut World, caster: Entity, primary: Entity) -> Vec<Entity> {
    let root = world
        .resource::<SpellStore>()
        .get(CRASH)
        .expect("native Crash Lightning selector")
        .clone();
    let effect = root
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Crash Lightning effect0");
    // SimC6113–6115 reevaluates the root's actual target list for each pulse, not only its original hits.
    targets::select_targets(world, caster, Some(primary), &root, effect, 0, false)
        .unwrap_or_else(|target| panic!("native Unleashed root selector {target}"))
}

fn cast_pack_pulse(world: &mut World, caster: Entity, primary: Entity, multiplier: f32, depth: u8) {
    if !targets::is_alive(world, caster) {
        return;
    }
    let pack = select_current_pack(world, caster, primary);
    let mut payload = world
        .resource::<SpellStore>()
        .get(DAMAGE)
        .expect("native Unleashed weapon strike")
        .clone();
    payload.cast_damage_multiplier = Some(multiplier);
    for target in pack {
        let occupies = combat::random::thread_rng().gen_range(0.0..1.0) < OCCUPANCY;
        if !occupies {
            continue;
        }
        if !spell_cast::launch_triggered_spell(world, caster, &payload, Some(target), depth) {
            warn!(?caster, ?target, "Native Unleashed weapon strike refused");
        }
    }
}
