//! Opt-in observation of actual creature cast inputs and admission results.
use super::*;
use crate::spell_cast::{CastFailure, SpellHistory};

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
