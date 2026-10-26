//! Shared Hellcaller producers and the native Blackened Soul counter clock.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason};
use combat::spell_cast::{
    effects::EffectCtx, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, Health};

const WITHER: u32 = 445474;
const BLACKENED_TALENT: u32 = 440043;
const COLLAPSE: u32 = 445731;
const BLACKENED: u32 = 445736;
const UA: u32 = 1259790;
const MALEVOLENCE: u32 = 430014;
const MALEVOLENCE_BUFF: u32 = 442726;
const MALEVOLENCE_DAMAGE: u32 = 446285;
const BLEAKHEART: u32 = 440051;
const SEEDS: u32 = 440055;
const XAVIUS: u32 = 440046;
// SimC76f8118 warlock.hpp RNG settings/init1441–1480, not damage coefficients.
const COLLAPSE_CHANCE: f64 = 0.23;
const BLEAKHEART_RATE: f64 = 0.15;
const SEEDS_INCREMENT: f64 = 0.240;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(after_application),
    SpellScriptHook::AfterResourceValues(after_cast),
    SpellScriptHook::LandedDamagePolicy(after_hit),
    SpellScriptHook::PeriodicPrimaryDamage(after_wither_tick),
    SpellScriptHook::ChannelPeriodic(periodic_trigger),
    SpellScriptHook::TargetDamageMultiplier(xavius_multiplier),
];

#[derive(Component)]
struct BleakheartAttempts {
    passive: u32,
    attempts: u32,
}
#[derive(Component)]
struct SeedsProgress {
    passive: u32,
    accumulated: f64,
}

fn owned_aura<'a>(
    world: &'a World,
    caster: Entity,
    target: Entity,
    spell: u32,
) -> Option<&'a AuraInstance> {
    let now = combat::auras::aura_now(world);
    world.get::<ActiveAuras>(target)?.iter().find(|aura| {
        aura.spell_id == spell
            && aura.caster == caster
            && aura.expires_at.is_none_or(|end| end > now)
    })
}

fn has_trait(world: &World, caster: Entity, spell: u32) -> bool {
    owned_aura(world, caster, caster, spell).is_some()
}

fn is_hellcaller(world: &World, caster: Entity) -> bool {
    let warlock = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| matches!(spec.0, 265 | 267));
    warlock && has_trait(world, caster, BLACKENED_TALENT)
}

fn native_base(world: &World, spell: u32, index: u8) -> f32 {
    // SimC effectN(n).base_value(): do not damage-modify dummy stack counts.
    world
        .resource::<SpellStore>()
        .get(spell)
        .expect("Hellcaller native spell")
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("Hellcaller native effect")
        .base_points
}

fn apply_native_aura(world: &mut World, caster: Entity, target: Entity, spell: u32) {
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn after_application(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !is_hellcaller(world, aura.caster) {
        return;
    }
    match aura.spell_id {
        // SimC76f8118 actions1563–1565 includes Fatal Echoes/seed-applied UA.
        // Additive application dispatch intentionally does not own UA AuraScripts.
        UA => grant_stacks(world, aura.caster, target, false),
        COLLAPSE => {
            // warlock.cpp103–126: permanent, frozen-stack counter, native period.
            let maximum = world
                .resource::<SpellStore>()
                .get(WITHER)
                .unwrap()
                .aura_options
                .as_ref()
                .unwrap()
                .cumulative_aura as u8;
            combat::auras::make_owned_counter_permanent(
                world,
                aura.caster,
                target,
                COLLAPSE,
                maximum,
            );
        }
        _ => {}
    }
}

fn after_hit(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    _amount: f32,
    periodic: bool,
) {
    // SimC76f8118 actions4328–4331/4807–4810: includes actual Havoc impacts.
    if !periodic && matches!(spell, Some(116858 | 17877)) && is_hellcaller(world, caster) {
        grant_stacks(world, caster, target, false);
    }
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    // Native TraitDefinition122451 projects talent430014 to visible442726.
    (spell == MALEVOLENCE_BUFF).then_some(cast_malevolence)
}

fn find_owned_wither_targets(world: &mut World, caster: Entity) -> Vec<Entity> {
    let now = combat::auras::aura_now(world);
    let mut query = world.query::<(Entity, &ActiveAuras, &Health)>();
    let alive = query
        .iter(world)
        .filter(|(_, _, health)| health.current > 0.0);
    alive
        .filter(|(_, auras, _)| {
            auras.iter().any(|aura| {
                aura.spell_id == WITHER
                    && aura.caster == caster
                    && aura.expires_at.is_none_or(|end| end > now)
            })
        })
        .map(|(entity, _, _)| entity)
        .collect()
}

fn cast_malevolence(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !is_hellcaller(world, caster) {
        return;
    }
    // SimC76f8118 actions1309–1316: the actual native442726 cast has
    // already applied its buff before this post-launch producer pass.
    for target in find_owned_wither_targets(world, caster) {
        grant_stacks(world, caster, target, true);
        cast_native_damage(world, caster, target, MALEVOLENCE_DAMAGE);
    }
}

fn grant_stacks(world: &mut World, caster: Entity, target: Entity, malevolence: bool) {
    let Some((current, maximum)) =
        owned_aura(world, caster, target, WITHER).map(|aura| (aura.stacks, aura.max_stacks))
    else {
        return;
    };
    // SimC76f8118 actions5534–5577: ordinary1, Malevolence native amount;
    // its buff adds a separate1; the global PRD adds at most one native bonus.
    let gain = if malevolence {
        native_base(world, MALEVOLENCE, 0) as u8
    } else {
        roll_ordinary_stack_gain(world, caster)
    };
    let stacks = current.saturating_add(gain).min(maximum);
    combat::auras::set_owned_stacks(world, caster, target, WITHER, stacks);
    if seed_gate(world, caster, target, stacks) {
        sync_collapse(world, caster, target, stacks);
        return;
    }
    let inactive = owned_aura(world, caster, target, COLLAPSE).is_none();
    if inactive && !malevolence && combat::random::random::<f64>() < COLLAPSE_CHANCE {
        sync_collapse(world, caster, target, stacks);
    }
}

fn roll_ordinary_stack_gain(world: &mut World, caster: Entity) -> u8 {
    let malevolence = if has_trait(world, caster, MALEVOLENCE_BUFF) {
        native_base(world, MALEVOLENCE, 1) as u8
    } else {
        0
    };
    let bleakheart = if roll_bleakheart(world, caster) {
        native_base(world, BLEAKHEART, 2) as u8
    } else {
        0
    };
    1 + malevolence + bleakheart
}

fn roll_bleakheart(world: &mut World, caster: Entity) -> bool {
    let Some(passive) = owned_aura(world, caster, caster, BLEAKHEART).map(|aura| aura.instance_id)
    else {
        return false;
    };
    if world
        .get::<BleakheartAttempts>(caster)
        .is_none_or(|state| state.passive != passive)
    {
        world.entity_mut(caster).insert(BleakheartAttempts {
            passive,
            attempts: 0,
        });
    }
    let constant = combat::random::accumulated_proc_constant(BLEAKHEART_RATE);
    let mut state = world.get_mut::<BleakheartAttempts>(caster).unwrap();
    state.attempts = state.attempts.saturating_add(1);
    let success = combat::random::random::<f64>() < constant * f64::from(state.attempts);
    if success {
        state.attempts = 0;
    }
    success
}

fn seed_gate(world: &World, caster: Entity, target: Entity, stacks: u8) -> bool {
    if !has_trait(world, caster, SEEDS) {
        return false;
    }
    let stack_gate = f32::from(stacks) >= native_base(world, SEEDS, 0);
    let health_gate = world.get::<Health>(target).is_some_and(|health| {
        stacks > 1 && health.current / health.max * 100.0 <= native_base(world, SEEDS, 1)
    });
    stack_gate || health_gate
}

fn after_wither_tick(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _amount: f32,
    _crit: bool,
) {
    if ctx.spell.id != WITHER || !is_hellcaller(world, ctx.caster) {
        return;
    }
    // SimC76f8118 actions1119–1149 rechecks Seed gates on every real Wither tick.
    if let Some(stacks) = owned_aura(world, ctx.caster, target, WITHER).map(|aura| aura.stacks)
        && seed_gate(world, ctx.caster, target, stacks)
    {
        sync_collapse(world, ctx.caster, target, stacks);
    }
}

fn sync_collapse(world: &mut World, caster: Entity, target: Entity, stacks: u8) {
    let previous = owned_aura(world, caster, target, COLLAPSE).map_or(0, |aura| aura.stacks);
    if previous == stacks {
        return;
    }
    // SimC76f8118 actions5586–5596: positive trigger refreshes the tick clock;
    // negative decrement preserves it. Runtime policy restores permanent/max99.
    if stacks > previous {
        apply_native_aura(world, caster, target, COLLAPSE);
    }
    combat::auras::set_owned_stacks(world, caster, target, COLLAPSE, stacks);
}

fn periodic_trigger(
    spell: u32,
    index: u8,
) -> Option<combat::spell_cast::script_registry::ContextHitHook> {
    (spell == COLLAPSE && index == 0).then_some(tick_blackened)
}

fn remove_collapse(world: &mut World, caster: Entity, target: Entity) {
    combat::auras::remove_auras(world, target, RemoveReason::Consumed, |aura| {
        aura.spell_id == COLLAPSE && aura.caster == caster
    });
}

fn tick_blackened(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let Some(stacks) = owned_aura(world, ctx.caster, target, WITHER).map(|aura| aura.stacks) else {
        remove_collapse(world, ctx.caster, target);
        return;
    };
    // SimC76f8118 actions1232–1272: decrement Wither before damage (Xavius),
    // then consume the frozen collapse budget after its impact.
    if stacks > 1 {
        combat::auras::set_owned_stacks(world, ctx.caster, target, WITHER, stacks - 1);
    }
    // Capture the live-dot inputs before native lethal-hit aura cleanup.
    let seed_weight = read_seed_weight(world, ctx.caster, target);
    cast_native_damage(world, ctx.caster, target, BLACKENED);
    consume_collapse_tick(world, ctx.caster, target);
    if let Some(weight) = seed_weight {
        trigger_seed_payout(world, ctx.caster, weight);
    }
}

fn consume_collapse_tick(world: &mut World, caster: Entity, target: Entity) {
    let wither = owned_aura(world, caster, target, WITHER).map_or(0, |aura| aura.stacks);
    let budget = owned_aura(world, caster, target, COLLAPSE).map_or(0, |aura| aura.stacks);
    if wither <= 1 || budget <= 1 {
        remove_collapse(world, caster, target);
    } else {
        combat::auras::set_owned_stacks(world, caster, target, COLLAPSE, budget - 1);
    }
}

fn cast_native_damage(world: &mut World, caster: Entity, target: Entity, spell: u32) {
    combat::spell_cast::cast_triggered(world, caster, spell, Some(target), 1);
}

fn xavius_multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != BLACKENED || !has_trait(world, caster, XAVIUS) {
        return 1.0;
    }
    // SimC76f8118 actions1218–1228: only Blackened Soul, after stack loss.
    let stacks = owned_aura(world, caster, target, WITHER).map_or(0, |aura| aura.stacks);
    1.0 + f32::from(stacks) * native_base(world, XAVIUS, 2) / 100.0
}

fn read_seed_weight(world: &mut World, caster: Entity, target: Entity) -> Option<f64> {
    if !has_trait(world, caster, SEEDS) {
        return None;
    }
    // SimC76f8118 init1461–1478: stack count before the decrement, active dots.
    let stacks_before = owned_aura(world, caster, target, WITHER)?.stacks + 1;
    let active = find_owned_wither_targets(world, caster).len();
    Some(f64::from(stacks_before).powf(-2.0 / 3.0) * (active as f64).powf(-3.0 / 4.0))
}

fn trigger_seed_payout(world: &mut World, caster: Entity, weight: f64) {
    // SimC76f8118 init1455–1478/proc_rng.cpp221–276: random initial progress,
    // weighted uniform increment and rollover, independent of other Warlock PRDs.
    if !roll_seed_progress(world, caster, SEEDS_INCREMENT * weight) {
        return;
    }
    // SimC76f8118 actions1274–1283: native spec-specific payout, not new shards.
    let spec = world.get::<ActiveSpec>(caster).unwrap().0;
    let (spell, stacks) = if spec == 265 {
        (1260269, 1)
    } else {
        (387263, native_base(world, SEEDS, 2) as u8)
    };
    let mut buff = world.resource::<SpellStore>().get(spell).unwrap().clone();
    buff.cast_aura_stacks = Some(stacks);
    combat::spell_cast::launch_triggered_spell(world, caster, &buff, Some(caster), 1);
}

fn roll_seed_progress(world: &mut World, caster: Entity, maximum: f64) -> bool {
    let passive = owned_aura(world, caster, caster, SEEDS)
        .unwrap()
        .instance_id;
    if world
        .get::<SeedsProgress>(caster)
        .is_none_or(|state| state.passive != passive)
    {
        let accumulated = combat::random::random::<f64>();
        world.entity_mut(caster).insert(SeedsProgress {
            passive,
            accumulated,
        });
    }
    let increment = combat::random::random::<f64>() * maximum;
    let mut state = world.get_mut::<SeedsProgress>(caster).unwrap();
    state.accumulated += increment;
    let success = state.accumulated >= 1.0;
    if success {
        state.accumulated -= 1.0;
    }
    success
}
