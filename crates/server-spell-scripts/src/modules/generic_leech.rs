//! Generic leech from effective outgoing damage and healing.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::protocol::{CombatLogEvent, CombatLogKind};

#[derive(Component)]
struct LeechPool {
    amount: f32,
    next: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::CombatStatusChanged(combat_changed),
    SpellScriptHook::HealthChangeObserved(observe),
    SpellScriptHook::AfterCastUpdate(update),
];

fn interval(world: &World, actor: Entity) -> f64 {
    let haste = world
        .get::<combat::stats::CharacterStats>(actor)
        .map_or(0.0, |stats| stats.haste_pct);
    1.5 / (1.0 + f64::from(haste) / 100.0).max(0.01)
}

fn combat_changed(world: &mut World, actor: Entity, active: bool) {
    if !active {
        world.entity_mut(actor).remove::<LeechPool>();
    } else if world.get::<combat::stats::CharacterStats>(actor).is_some() {
        // SimC76f8118 player.cpp470–495 schedules leech every hasted base GCD.
        let next = spell_cast::now(world) + interval(world, actor);
        world
            .entity_mut(actor)
            .insert(LeechPool { amount: 0.0, next });
    }
}

fn observe(world: &mut World, event: &CombatLogEvent) {
    let (Some(source), Some(target)) = (
        event.source.and_then(Entity::try_from_bits),
        event.target.and_then(Entity::try_from_bits),
    ) else {
        return;
    };
    if event.kind != CombatLogKind::Damage
        && (event.kind != CombatLogKind::Heal || source == target)
    {
        return;
    }
    // SimC heal.cpp280–289 excludes self-healing; amounts exclude overheal/overkill.
    let effective = (event.amount - event.overflow).max(0) as f32;
    let leech = world
        .get::<combat::stats::CharacterStats>(source)
        .map_or(0.0, |stats| stats.leech_pct)
        .max(0.0);
    if effective > 0.0 && leech > 0.0 {
        if let Some(mut pool) = world.get_mut::<LeechPool>(source) {
            pool.amount += effective * leech / 100.0;
        }
    }
}

fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let actors: Vec<_> = world
        .query::<(Entity, &LeechPool)>()
        .iter(world)
        .filter(|(_, pool)| pool.next <= now)
        .map(|(actor, _)| actor)
        .collect();
    for actor in actors {
        let next = now + interval(world, actor);
        let amount = {
            let mut pool = world.get_mut::<LeechPool>(actor).expect("leech pool");
            pool.next = next;
            std::mem::take(&mut pool.amount)
        };
        if amount <= 0.0 {
            continue;
        }
        let Some(spell) = spell_cast::load_triggered_spell(world, 143924, 1) else {
            error!(?actor, "native Leech spell missing");
            continue;
        };
        let ctx = EffectCtx {
            caster: actor,
            spell: &spell,
            depth: 1,
            dest: None,
            chain_multiplier: 1.0,
        };
        spell_cast::effects::apply_heal(world, &ctx, actor, amount, false, false);
    }
}
