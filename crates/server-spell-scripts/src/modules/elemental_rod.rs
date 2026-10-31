//! Elemental Lightning Rod: spender application and resolved lightning copies.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::{combat, enhancement::enhancement_rod};
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook, targets};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::AfterResolvedHits(after_resolution),
];

const TALENT: u32 = 210689;
const ROD: u32 = 197209;

fn declared_reach(
    _world: &World,
    _caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    match spell.id {
        188196 | 45284 | 188443 | 45297 | 463351 => {
            combat::spell_cast::script_registry::ScriptReach::EngagedOnly
        }
        61882 | 462620 => {
            let effect = spell
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Earthquake effect");
            combat::spell_cast::script_registry::ScriptReach::target(targets::radius(effect, 0))
        }
        _ => Default::default(),
    }
}

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 262)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(TALENT))
}

fn after_resolution(world: &mut World, ctx: &EffectCtx<'_>) {
    if !selected(world, ctx.caster) {
        return;
    }
    match ctx.spell.id {
        // SimC76f8118 sc_shaman.cpp6335–6343,6703–6709,7342–7348,
        // 7511–7517,9849–9855: one resolved cast is copied to every existing Rod.
        188196 | 45284 | 188443 | 45297 | 463351 => {
            let amount = spell_cast::resolving_damage(world, ctx)
                .expect("Lightning Rod requires resolved lightning damage")
                .iter()
                .map(|(_, amount)| amount)
                .sum();
            enhancement_rod::mirror_existing_rods(world, ctx, amount);
        }
        // sc_shaman.cpp7727–7742,8238–8245: each successful spender hit applies or refreshes.
        8042 | 117014 => {
            let targets: Vec<Entity> = spell_cast::resolving_damage(world, ctx)
                .expect("Lightning Rod requires resolved spender damage")
                .iter()
                .filter_map(|(target, amount)| (*amount > 0.0).then_some(*target))
                .collect();
            for target in targets {
                schedule_rod(world, ctx, target);
            }
        }
        // sc_shaman.cpp7976–8004: prefer an unmarked primary; otherwise choose
        // another unmarked target in the actual Earthquake area, without refresh.
        61882 | 462620 => apply_quake_rod(world, ctx),
        _ => {}
    }
}

fn apply_quake_rod(world: &mut World, ctx: &EffectCtx<'_>) {
    let primary = ctx.primary_target();
    let center = ctx
        .spell
        .cast_destination
        .map(Vec3::from_array)
        .or_else(|| primary.and_then(|unit| targets::position(world, unit)));
    let Some(center) = center else {
        warn!(caster = ?ctx.caster, "Earthquake Lightning Rod has no center");
        return;
    };
    let effect = ctx
        .spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Earthquake native radius effect");
    let radius = targets::radius(effect, 0);
    let candidates = targets::area_units(
        world,
        ctx.caster,
        center,
        radius,
        targets::Relation::Enemy,
        0,
    );
    let available: Vec<Entity> = candidates
        .into_iter()
        .filter(|target| !enhancement_rod::owns_rod(world, *target, ctx.caster))
        .collect();
    let target = primary
        .filter(|target| available.contains(target))
        .or_else(|| {
            (!available.is_empty())
                .then(|| available[combat::random::thread_rng().gen_range(0..available.len())])
        });
    if let Some(target) = target {
        schedule_rod(world, ctx, target);
    }
}

fn schedule_rod(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let delay_ms = combat::random::thread_rng()
        .gen_range(10.0_f64..100.0)
        .ceil() as u32;
    spell_cast::queued_triggers::schedule_full_trigger(world, ctx, target, ROD, delay_ms);
}
