//! Opt-in observation of actual creature cast inputs and admission results.
use super::*;
use crate::spell_cast::{CastFailure, SpellHistory};

#[derive(Debug)]
pub(super) struct Admission {
    pub spell: u32,
    pub time: f32,
    pub result: Result<(), CastFailure>,
    pub on_cooldown: bool,
}

#[derive(Resource)]
pub(super) struct CastAdmissions {
    pub actor: Entity,
    pub origin: f64,
    pub entries: Vec<Admission>,
}

/// One actor's fixture RNG seed, applied only at natural ground release.
#[derive(Component)]
pub(super) struct GroundCombatSeed(pub u64);

/// Opt-in fixture evidence; never installed in production or ordinary test apps.
#[derive(Resource)]
pub(super) struct AiTickTrace {
    pub actor: Entity,
    pub origin: f64,
    pub frames: Vec<AiTickFrame>,
}

#[derive(serde::Serialize)]
pub(super) struct AiTickFrame {
    time: f64,
    diff_ms: u32,
    ai_clock_ms: u64,
    victim: Option<u64>,
    engaged: bool,
    cast_transition: &'static str,
    pub(super) cast: Option<shared::casting::CastState>,
}

/// Capture after the real Herald AI tick, including a paused combat EventMap.
pub(in crate::scripted_ai) fn record_ai_tick(
    world: &mut World,
    actor: Entity,
    ai_clock_ms: u64,
    diff_ms: u32,
) {
    let Some(trace) = world.get_resource::<AiTickTrace>() else {
        return;
    };
    if trace.actor != actor {
        return;
    }
    let cast = world.get::<shared::casting::CastState>(actor).cloned();
    let previous = trace.frames.last().and_then(|frame| frame.cast.as_ref());
    let cast_transition = match (previous, cast.as_ref()) {
        (None, Some(_)) => "start",
        (Some(_), None) => "end",
        (Some(before), Some(after)) if before.spell_id != after.spell_id => "replace",
        _ => "unchanged",
    };
    let origin = trace.origin;
    let ctx = Ctx { world, me: actor };
    let frame = AiTickFrame {
        time: ctx.world.resource::<Time>().elapsed_secs_f64() - origin,
        diff_ms,
        ai_clock_ms,
        victim: ctx.victim().map(Entity::to_bits),
        engaged: ctx.is_engaged(),
        cast_transition,
        cast,
    };
    world.resource_mut::<AiTickTrace>().frames.push(frame);
}

pub(in crate::scripted_ai) fn record(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    result: &Result<(), CastFailure>,
) {
    let Some(observer) = world.get_resource::<CastAdmissions>() else {
        return;
    };
    if observer.actor != caster {
        return;
    }
    let time = (world.resource::<Time>().elapsed_secs_f64() - observer.origin) as f32;
    let spell = world
        .resource::<SpellStore>()
        .get(spell_id)
        .expect("observed source spell");
    let now = crate::spell_cast::history_now(world);
    let on_cooldown = world
        .get::<SpellHistory>(caster)
        .is_some_and(|history| history.on_cooldown(spell, now));
    world
        .resource_mut::<CastAdmissions>()
        .entries
        .push(Admission {
            spell: spell_id,
            time,
            result: result.clone(),
            on_cooldown,
        });
}
