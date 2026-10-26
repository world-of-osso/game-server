//! SimC76f8118 sc_shaman.cpp9994–10007/12437–12484: Tempest buffs and Arc's delayed chain.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::auras::procs::{self, AuraEvent};
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{self, ActiveAuras, AuraInstance};
use crate::class_progression::KnownSpells;
use crate::random::Rng;
use crate::spell_cast::{self, effects::EffectCtx, targets};
use crate::spell_info::SpellStore;

const ARC_TALENT: u32 = 455096;
const ARC_DISCHARGE: u32 = 470532;
const SWELL_TALENT: u32 = 455088;
const STORM_SWELL: u32 = 455089;
const CHAIN: u32 = 188443;
const RIDE_CHAIN: u32 = 211094;
const MIN_DELAY_SECONDS: f64 = 0.325;
const MAX_DELAY_SECONDS: f64 = 0.425;

struct Discharge {
    target: Entity,
    spell: u32,
    ready_at: f64,
    depth: u8,
}

#[derive(Component, Default)]
struct Pending(Vec<Discharge>);

pub(super) fn register(scripts: &mut AuraScripts) {
    for spell in [SWELL_TALENT, ARC_DISCHARGE] {
        scripts.0.insert(
            spell,
            AuraScript {
                check_proc: Some(allow_other_specs),
                ..Default::default()
            },
        );
    }
}

fn enhancement(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 263)
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(talent))
}

fn allow_other_specs(world: &World, aura: &AuraInstance, _event: &AuraEvent) -> bool {
    // Elemental's separate Stormkeeper/Storm Swell policy is outside this263 slice.
    !enhancement(world, aura.caster)
}

pub(super) fn grant(world: &mut World, caster: Entity) {
    if !enhancement(world, caster) {
        return;
    }
    // SimC9994–10007/12660–12667: one native trigger per actual Tempest execution.
    for (talent, buff) in [(SWELL_TALENT, STORM_SWELL), (ARC_TALENT, ARC_DISCHARGE)] {
        if selected(world, caster, talent) {
            spell_cast::cast_triggered(world, caster, buff, Some(caster), 1);
        }
    }
}

fn find_arc_instance(world: &World, caster: Entity) -> Option<u32> {
    world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == ARC_DISCHARGE && aura.caster == caster)
            .map(|aura| aura.instance_id)
    })
}

pub(in crate::spell_cast) fn after_chain(world: &mut World, ctx: &EffectCtx) {
    if !matches!(ctx.spell.id, CHAIN | RIDE_CHAIN) {
        return;
    }
    let eligible = enhancement(world, ctx.caster) && selected(world, ctx.caster, ARC_TALENT);
    if !eligible {
        return;
    }
    let Some(instance) = find_arc_instance(world, ctx.caster) else {
        return;
    };
    let recipients = super::super::resolving_targets(world, ctx)
        .expect("native Arc Discharge retains its actual chain recipients");
    let Some(target) = recipients.last().copied() else {
        return;
    };
    // SimC12460–12484: one stack at the last impact, with the native400ms ICD.
    if procs::start_policy_proc(world, ctx.caster, instance).is_none() {
        return;
    }
    auras::spend_stack(world, ctx.caster, instance);
    schedule(world, ctx, target);
}

fn schedule(world: &mut World, ctx: &EffectCtx, target: Entity) {
    // SimC12462–12466: uniform325–425ms permits self-procs only beyond the native ICD.
    let delay = crate::random::thread_rng().gen_range(MIN_DELAY_SECONDS..MAX_DELAY_SECONDS);
    let discharge = Discharge {
        target,
        spell: ctx.spell.id,
        ready_at: spell_cast::now(world) + delay,
        depth: ctx.depth + 1,
    };
    if let Some(mut pending) = world.get_mut::<Pending>(ctx.caster) {
        pending.0.push(discharge);
    } else {
        world
            .entity_mut(ctx.caster)
            .insert(Pending(vec![discharge]));
    }
}

fn take_due(world: &mut World) -> Vec<(Entity, Discharge)> {
    let now = spell_cast::now(world);
    let mut due = Vec::new();
    for (caster, mut pending) in world.query::<(Entity, &mut Pending)>().iter_mut(world) {
        due.extend(
            pending
                .0
                .extract_if(.., |event| event.ready_at <= now)
                .map(|event| (caster, event)),
        );
    }
    due
}

pub(in crate::spell_cast) fn resolve_due(world: &mut World) {
    for (caster, discharge) in take_due(world) {
        cast_discharge(world, caster, discharge);
    }
}

fn cast_discharge(world: &mut World, caster: Entity, discharge: Discharge) {
    // SimC12468–12483 never substitutes another target for a sleeping recipient.
    if !targets::is_alive(world, discharge.target) {
        return;
    }
    let mut payload = world
        .resource::<SpellStore>()
        .get(discharge.spell)
        .expect("native Arc Discharge chain payload")
        .clone();
    payload.cast_proc_mods = super::snapshot_proc_modifiers(world, caster, &payload);
    payload.cast_charge_mods = Some(
        auras::spell_mods::snapshot_charged_modifiers(world, caster, &payload)
            .into_iter()
            .filter(|modifier| {
                discharge.spell != RIDE_CHAIN
                    || modifier.aura_spell_id != super::maelstrom_weapon::MODIFIER
            })
            .collect(),
    );
    // SimC6587–6597/6631–6643: ordinary AD benefits from live MW, RTL does not; neither spends it.
    spell_cast::launch_triggered_spell(
        world,
        caster,
        &payload,
        Some(discharge.target),
        discharge.depth,
    );
}
