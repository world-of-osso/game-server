//! Antoran Inquisitor's native Mind Sear begins with an immediate tick.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OnCast(initial_tick),
    SpellScriptHook::AfterQueuedTriggers(repeat_casts),
];

#[derive(Component)]
struct NextMindSear { due: f64, target: Entity }

fn initial_tick(world: &mut World, actor: Entity, spell: u32, depth: u8) {
    if spell != 1280457
        || world
            .get::<Npc>(actor)
            .is_none_or(|npc| npc.template_id != 258242)
    {
        return;
    }
    let Some(owner) = world
        .get::<SummonedCreature>(actor)
        .map(|summon| summon.owner)
    else {
        return;
    };
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0)
    else {
        return;
    };
    // SimC76f8118 pets.cpp1978–2002: the unhasted channel ticks at zero;
    // the pet schedules its next hasted execute while that channel runs.
    combat::spell_cast::cast_triggered(world, actor, 1280460, Some(target), depth + 1);
    let spell = world.resource::<combat::spell_info::SpellStore>().get(1280457)
        .expect("native Mind Sear");
    let cast_ms = combat::spell_cast::cast_time_ms(world, actor, spell);
    let due = combat::spell_cast::now(world) + f64::from(cast_ms) / 1000.0;
    world.entity_mut(actor).insert(NextMindSear { due, target });
}

fn repeat_casts(world: &mut World) {
    let now = combat::spell_cast::now(world);
    let due: Vec<_> = world.query::<(Entity, &NextMindSear)>().iter(world)
        .filter(|(_, cast)| cast.due <= now)
        .map(|(actor, cast)| (actor, cast.target)).collect();
    for (actor, target) in due {
        world.entity_mut(actor).remove::<NextMindSear>();
        if !combat::spell_cast::try_cast_triggered(world, actor, 1280457, Some(target), 1) {
            warn!(?actor, ?target, "scheduled Mind Sear refused");
        }
    }
}
