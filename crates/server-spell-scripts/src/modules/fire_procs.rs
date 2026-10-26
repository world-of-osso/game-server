//! Fire's sourced Hot Streak producers; all payloads and amounts remain native.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, AuraInstance};
use combat::class_progression::KnownSpells;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::{reduce_cooldown, script_effect_value};
use combat::spell_cast::{self, effects::EffectCtx, queued_triggers};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::FireAuraRegistration(register),
    SpellScriptHook::BeforeProcLaunch(before_hot_streak_launch),
    SpellScriptHook::AfterFireDirectHit(fire_feedback_hit),
    SpellScriptHook::AfterFireDirectHit(refund_from_the_ashes),
    SpellScriptHook::PeriodicDamageFinished(roll_heat_shimmer),
    SpellScriptHook::AfterFireOutcomeHit(fire_meteorite_hit),
    SpellScriptHook::OnCast(on_fire_cast),
    SpellScriptHook::TargetDamageMultiplier(meteor_primary_multiplier),
];

const COMBUSTION: u32 = 190319;
const FIRED_UP_TALENT: u32 = 1257343;
const FIRED_UP: u32 = 1257350;
const SPHERES_TALENT: u32 = 448601;
const SPHERE: u32 = 448604;
const SPHERE_CAP: u32 = 11;
const GLORIOUS_TALENT: u32 = 449394;
const GLORIOUS: u32 = 451073;
const METEORITE_MISSILE: u32 = 449560;
const CASCADE_TALENT: u32 = 449293;
const CASCADE: u32 = 449314;
const MEMORY: u32 = 449619;
const HYPERTHERMIA: u32 = 383874;
const HYPERTHERMIA_DAMAGE: u32 = 1242220;
const OVERFLOWING_TALENT: u32 = 390218;
const OVERFLOWING: u32 = 394195;
const FEVERED_TALENT: u32 = 383810;
const FEVERED: u32 = 383811;
const CONTROLLED_TALENT: u32 = 383669;
const CONTROLLED: u32 = 453268;
const MAGE_FAMILY: u32 = 3;
const PYROCOSM: u32 = 1260673;
const METEORITE_IMPACT: u32 = 449569;
const FROM_THE_ASHES: u32 = 342344;
const HEAT_SHIMMER_TALENT: u32 = 457735;
const HEAT_SHIMMER: u32 = 458964;
const IGNITE: u32 = 12654;
const SCORCH: u32 = 2948;

#[derive(Component, Default)]
struct FireProcState {
    sphere_attempts: u32,
    combustion_procs: usize,
    ashes_ready_at: Option<f64>,
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(talent))
}

fn ensure_state(world: &mut World, caster: Entity) {
    if world.get::<FireProcState>(caster).is_none() {
        world.entity_mut(caster).insert(FireProcState::default());
    }
}

/// SimC76f8118 sc_mage.cpp2406–2412: these producers precede damage execution.
pub(crate) fn before_hot_streak_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(
        spell.id,
        super::hot_streak::PYROBLAST
            | super::hot_streak::FLAMESTRIKE
            | super::hot_streak::NATIVE_FLAMESTRIKE
    ) || super::hot_streak::admitted_hot_streak(world, caster, spell).is_none()
    {
        return;
    }
    ensure_state(world, caster);
    roll_fired_up(world, caster);
    roll_spellfire_sphere(world, caster);
}

/// SimC76f8118 sc_mage.cpp2425–2430/2446–2447: Cascade follows the spend;
/// Hyperthermia's damage stack follows every completed Hot Streak-class spell.
pub(super) fn after_hot_streak_cast(world: &mut World, caster: Entity, spent_hot_streak: bool) {
    if spent_hot_streak && selected(world, caster, CASCADE_TALENT) {
        spell_cast::cast_triggered(world, caster, CASCADE, Some(caster), 1);
    }
    if auras::has_aura(world, caster, HYPERTHERMIA) {
        spell_cast::cast_triggered(world, caster, HYPERTHERMIA_DAMAGE, Some(caster), 1);
    }
}

fn roll_fired_up(world: &mut World, caster: Entity) {
    if !selected(world, caster, FIRED_UP_TALENT) {
        return;
    }
    let combustion = auras::has_aura(world, caster, COMBUSTION);
    // SimC76f8118 sc_mage.cpp7205–7212: pinned Combustion proc curve,
    // including its default bugs=1 behavior; outside it use native Effect1.
    const COMBUSTION_CHANCE: &[f32] = &[89.7, 61.6, 37.3, 20.1, 10.9, 7.28];
    let count = world.get::<FireProcState>(caster).unwrap().combustion_procs;
    let chance = if combustion {
        COMBUSTION_CHANCE.get(count).copied().unwrap_or(0.0)
    } else {
        script_effect_value(world, caster, FIRED_UP_TALENT, 0)
    };
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    apply_fired_up(world, caster);
    if combustion {
        world
            .get_mut::<FireProcState>(caster)
            .unwrap()
            .combustion_procs += 1;
    }
}

fn apply_fired_up(world: &mut World, caster: Entity) {
    // sc_mage.cpp7216–7224: native modified effect2 is milliseconds and
    // effect3 extends the existing Combustion without resetting its clock.
    spell_cast::cast_triggered(world, caster, FIRED_UP, Some(caster), 1);
    let reduction_ms = script_effect_value(world, caster, FIRED_UP_TALENT, 1);
    reduce_cooldown(world, caster, 108853, f64::from(reduction_ms) / 1000.0);
    let extension_ms = script_effect_value(world, caster, FIRED_UP_TALENT, 2).max(0.0) as u32;
    auras::extend_owned_duration(world, caster, caster, &[COMBUSTION], extension_ms);
    super::fire_phoenix::extend(world, caster, extension_ms);
}

fn roll_spellfire_sphere(world: &mut World, caster: Entity) {
    if !selected(world, caster, SPHERES_TALENT) {
        return;
    }
    // SimC76f8118 sc_mage.cpp6633–6636/7310–7320: Fire uses effect2,
    // with the pinned11-attempt PRD cap, not an independent fixed-chance roll.
    let probability = f64::from(script_effect_value(world, caster, SPHERES_TALENT, 1)) / 100.0;
    let constant = combat::random::capped_accumulated_proc_constant(probability, SPHERE_CAP);
    let mut state = world.get_mut::<FireProcState>(caster).unwrap();
    state.sphere_attempts += 1;
    let chance = if state.sphere_attempts >= SPHERE_CAP {
        1.0
    } else {
        constant * f64::from(state.sphere_attempts)
    };
    if combat::random::random::<f64>() >= chance {
        return;
    }
    state.sphere_attempts = 0;
    spell_cast::cast_triggered(world, caster, SPHERE, Some(caster), 1);
    // SimC76f8118 sc_mage.cpp7316–7319: the same Sphere success produces
    // selected Glorious Incandescence; it has no second independent roll.
    if selected(world, caster, GLORIOUS_TALENT) {
        spell_cast::cast_triggered(world, caster, GLORIOUS, Some(caster), 1);
    }
}

/// SimC76f8118 sc_mage.cpp4577–4580/2003–2012: a Fire Blast hit with
/// Glorious launches the native count, staggered75ms; native449560 supplies travel.
pub(crate) fn fire_meteorite_hit(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if ctx.spell.id != 108853
        || !auras::has_aura(world, ctx.caster, GLORIOUS)
        || !selected(world, ctx.caster, GLORIOUS_TALENT)
    {
        return;
    }
    let count = script_effect_value(world, ctx.caster, GLORIOUS_TALENT, 0).max(0.0) as u32;
    schedule_meteorites(world, ctx, target, count);
}

fn schedule_meteorites(world: &mut World, ctx: &EffectCtx, target: Entity, count: u32) {
    let Some(position) = world.get::<shared::components::Position>(target).copied() else {
        return;
    };
    for ordinal in 0..count {
        queued_triggers::schedule_destination_trigger(
            world,
            ctx,
            queued_triggers::DestinationTrigger {
                target,
                spell_id: METEORITE_MISSILE,
                delay_ms: ordinal * 75,
                destination: [position.x, position.y, position.z],
            },
        );
    }
}

/// SimC76f8118 sc_mage.cpp1890–1901: real direct Mage hits update
/// crit feedback after their hit batch, not before later targets roll damage.
pub(crate) fn fire_feedback_hit(world: &mut World, ctx: &EffectCtx, target: Entity, crit: bool) {
    let mage_spell = ctx
        .spell
        .class_options
        .as_ref()
        .is_some_and(|options| options.spell_class_set == MAGE_FAMILY);
    if !mage_spell {
        return;
    }
    // sc_mage.cpp3819: Fire's Overflowing Energy eligibility is Fireball
    // (including the selected Frostfire replacement), not every Hot Streak producer.
    if matches!(ctx.spell.id, 133 | 431044) && selected(world, ctx.caster, OVERFLOWING_TALENT) {
        queued_triggers::schedule_buff_outcome(world, ctx.caster, OVERFLOWING, !crit);
    }
    if selected(world, ctx.caster, FEVERED_TALENT) {
        queued_triggers::schedule_buff_outcome(world, ctx.caster, FEVERED, crit);
    }
    apply_controlled_destruction(world, ctx, target);
    // SimC76f8118 sc_mage.cpp3845–3846/2003–2012: Pyrocosm Fireball
    // hits independently roll native effect2, then schedule one real Meteorite.
    if matches!(ctx.spell.id, 133 | 431044)
        && selected(world, ctx.caster, PYROCOSM)
        && selected(world, ctx.caster, GLORIOUS_TALENT)
    {
        let chance = script_effect_value(world, ctx.caster, PYROCOSM, 1);
        if combat::random::random::<f32>() * 100.0 < chance {
            schedule_meteorites(world, ctx, target, 1);
        }
    }
}

fn roll_heat_shimmer(world: &mut World, ctx: &EffectCtx) {
    if ctx.spell.id != IGNITE || !selected(world, ctx.caster, HEAT_SHIMMER_TALENT) {
        return;
    }
    // SimC76f8118 sc_mage.cpp2557–2559/6439–6441: actual Ignite ticks
    // attempt the talent's native RPPM, except while Scorch is being cast.
    let scorching = world
        .get::<shared::casting::CastState>(ctx.caster)
        .is_some_and(|cast| cast.spell_id == SCORCH);
    if !scorching && auras::procs::try_native_policy_proc(world, ctx.caster, HEAT_SHIMMER_TALENT) {
        spell_cast::cast_triggered(world, ctx.caster, HEAT_SHIMMER, Some(ctx.caster), 1);
    }
}

fn refund_from_the_ashes(world: &mut World, ctx: &EffectCtx, _target: Entity, _crit: bool) {
    // SimC76f8118 sc_mage.cpp2116–2120: eligible actual hits restore
    // native Fire Blast recharge progress, respecting the talent's native ICD.
    // Producers:3689,3821,4557,4905,5040; Pyromaniac copies are excluded.
    let eligible = matches!(ctx.spell.id, 133 | 431044 | 108853 | 11366 | 2948 | 31661);
    if !eligible || !selected(world, ctx.caster, FROM_THE_ASHES) {
        return;
    }
    ensure_state(world, ctx.caster);
    let now = auras::aura_now(world);
    let ready_at = world
        .get::<FireProcState>(ctx.caster)
        .unwrap()
        .ashes_ready_at;
    if ready_at.is_some_and(|deadline| now < deadline) {
        return;
    }
    let icd_ms = world
        .resource::<SpellStore>()
        .get(FROM_THE_ASHES)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native From the Ashes proc options")
        .proc_category_recovery_ms;
    world
        .get_mut::<FireProcState>(ctx.caster)
        .unwrap()
        .ashes_ready_at = Some(now + f64::from(icd_ms) / 1000.0);
    let adjustment_ms = script_effect_value(world, ctx.caster, FROM_THE_ASHES, 0);
    reduce_cooldown(
        world,
        ctx.caster,
        108853,
        -f64::from(adjustment_ms) / 1000.0,
    );
}

fn on_fire_cast(world: &mut World, caster: Entity, spell_id: u32, _depth: u8) {
    if spell_id == METEORITE_IMPACT && selected(world, caster, PYROCOSM) {
        // SimC76f8118 sc_mage.cpp4783–4792: once per Meteorite execution,
        // not per recipient; the native effect4 value is milliseconds.
        let reduction = script_effect_value(world, caster, PYROCOSM, 3);
        reduce_cooldown(world, caster, 108853, f64::from(reduction) / 1000.0);
    }
}

fn meteor_primary_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    const METEOR_IMPACT: u32 = 351140;
    const DEEP_IMPACT: u32 = 1254857;
    if spell.id != METEOR_IMPACT || !selected(world, caster, DEEP_IMPACT) {
        return 1.0;
    }
    let Some(destination) = spell.cast_destination else {
        return 1.0;
    };
    // SimC76f8118 sc_mage.cpp4664–4668: native Deep Impact amplifies
    // only the closest enemy to the authored destination, not the entire AoE.
    if closest_meteor_target(world, caster, Vec3::from_array(destination)) != Some(target) {
        return 1.0;
    }
    1.0 + script_effect_value(world, caster, DEEP_IMPACT, 0) / 100.0
}

fn closest_meteor_target(world: &World, caster: Entity, destination: Vec3) -> Option<Entity> {
    use combat::maps::InMap;
    use combat::spell_cast::targets::{Relation, relation_matches};
    use shared::components::{Health, Position};
    let map = world.get::<InMap>(caster)?;
    let mut query = world.try_query::<(Entity, &Position, &Health, &InMap)>()?;
    query
        .iter(world)
        .filter(|(entity, _, health, candidate_map)| {
            health.current > 0.0
                && *candidate_map == map
                && relation_matches(world, caster, *entity, Relation::Enemy)
        })
        .map(|(entity, position, _, _)| {
            let distance =
                Vec3::new(position.x, position.y, position.z).distance_squared(destination);
            (entity, distance)
        })
        .min_by(|(left, left_distance), (right, right_distance)| {
            left_distance
                .total_cmp(right_distance)
                .then_with(|| left.to_bits().cmp(&right.to_bits()))
        })
        .map(|(entity, _)| entity)
}

fn apply_controlled_destruction(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if !matches!(ctx.spell.id, 133 | 431044 | 11366 | 2120 | 1254851)
        || !selected(world, ctx.caster, CONTROLLED_TALENT)
    {
        return;
    }
    // sc_mage.cpp2457/3843/5648–5650: one owned target stack per hit;
    // the native talent supplies the percent, scaled by the sourced one-tenth rule.
    let percent = script_effect_value(world, ctx.caster, CONTROLLED_TALENT, 0) * 0.1;
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        CONTROLLED,
        Some(target),
        1,
        percent,
    );
}

pub(crate) fn register(scripts: &mut auras::scripts::AuraScripts) {
    scripts.0.insert(
        COMBUSTION,
        auras::scripts::AuraScript {
            on_apply: Some(reset_combustion_procs),
            on_remove: Some(end_fiery_rush),
            on_natural_end: Some(start_hyperthermia),
            ..Default::default()
        },
    );
    scripts.0.insert(
        HYPERTHERMIA,
        auras::scripts::AuraScript {
            on_remove: Some(remove_hyperthermia_damage),
            ..Default::default()
        },
    );
    scripts.0.insert(
        GLORIOUS,
        auras::scripts::AuraScript {
            check_proc: Some(check_glorious_fire_blast),
            ..Default::default()
        },
    );
}

fn check_glorious_fire_blast(
    _world: &World,
    _aura: &AuraInstance,
    event: &auras::procs::AuraEvent,
) -> bool {
    // SimC76f8118 sc_mage.cpp4577–4580: only Fire Blast spends this
    // charge; the broad native harmful-spell mask also matches Pyroblast.
    event.spell_id == Some(108853)
}

fn end_fiery_rush(
    world: &mut World,
    caster: Entity,
    _aura: &AuraInstance,
    reason: auras::RemoveReason,
) {
    // SimC76f8118 sc_mage.cpp1453–1457: the native permanent Fiery Rush
    // payload lasts only while Combustion is active; do not leave it after expiry.
    auras::remove_auras(world, caster, reason, |aura| {
        aura.spell_id == 383637 && aura.caster == caster
    });
}

fn reset_combustion_procs(world: &mut World, caster: Entity, aura: &AuraInstance) {
    // SimC76f8118 sc_mage.cpp1450: each new Combustion resets its proc curve.
    ensure_state(world, caster);
    world
        .get_mut::<FireProcState>(caster)
        .unwrap()
        .combustion_procs = 0;
    super::fire_phoenix::invoke(world, caster, aura);
}

fn start_hyperthermia(world: &mut World, caster: Entity, _aura: &AuraInstance) {
    if !selected(world, caster, MEMORY) {
        return;
    }
    // SimC76f8118 sc_mage.cpp1268–1275: Memory starts at the end of the
    // Phoenix/Combustion lifetime, even without an active Phoenix action.
    let milliseconds = script_effect_value(world, caster, MEMORY, 1).max(0.0) as u32;
    spell_cast::cast_triggered(world, caster, HYPERTHERMIA, Some(caster), 1);
    auras::set_owned_duration(world, caster, caster, HYPERTHERMIA, milliseconds);
}

fn remove_hyperthermia_damage(
    world: &mut World,
    caster: Entity,
    _aura: &AuraInstance,
    reason: auras::RemoveReason,
) {
    // SimC76f8118 sc_mage.cpp6492–6494: its stacked bonus ends with Hyperthermia.
    auras::remove_auras(world, caster, reason, |aura| {
        aura.spell_id == HYPERTHERMIA_DAMAGE && aura.caster == caster
    });
}
