//! Vengeance Frailty target stacks and delayed healing.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::protocol::{CombatLogEvent, CombatLogKind};

#[derive(Component)]
struct FrailtyPool {
    amount: f32,
    next: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::CombatStatusChanged(combat_changed),
    SpellScriptHook::HealthChangeObserved(observe),
    SpellScriptHook::AfterCastUpdate(update),
];

fn combat_changed(world: &mut World, actor: Entity, active: bool) {
    if !active {
        world.entity_mut(actor).remove::<FrailtyPool>();
    } else if world
        .get::<combat::class_progression::KnownSpells>(actor)
        .is_some_and(|known| known.contains(389958))
    {
        // SimC76f8118 sc_demon_hunter.cpp9462–9495 randomizes the first one-second tick.
        let next = spell_cast::now(world) + combat::random::random::<f64>();
        world
            .entity_mut(actor)
            .insert(FrailtyPool { amount: 0.0, next });
    }
}

fn observe(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage {
        return;
    }
    let (Some(source), Some(target)) = (
        event.source.and_then(Entity::try_from_bits),
        event.target.and_then(Entity::try_from_bits),
    ) else {
        return;
    };
    let multiplier = world
        .get::<combat::auras::ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 247456 && aura.caster == source)
        })
        .and_then(|aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == 0)
                .map(|effect| effect.amount * f32::from(aura.stacks) / 100.0)
        })
        .unwrap_or(0.0);
    if let Some(mut pool) = world.get_mut::<FrailtyPool>(source) {
        // SimC76f8118 sc_demon_hunter.cpp2578–2591 accumulates damage against owned Frailty.
        pool.amount += (event.amount - event.overflow).max(0) as f32 * multiplier;
    }
    if !event.periodic
        && matches!(event.spell_id, Some(204598 | 228478 | 247455))
        && world
            .get::<combat::class_progression::KnownSpells>(source)
            .is_some_and(|known| known.contains(389958))
    {
        // SimC76f8118 sc_demon_hunter.cpp4440–4443, 5346–5349, 8161–8164.
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster: source,
                target,
                spell_id: 247456,
                effect_mask: u32::MAX,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
}

fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let actors: Vec<_> = world
        .query::<(Entity, &FrailtyPool)>()
        .iter(world)
        .filter(|(_, pool)| pool.next <= now)
        .map(|(actor, _)| actor)
        .collect();
    for actor in actors {
        let amount = {
            let mut pool = world.get_mut::<FrailtyPool>(actor).expect("Frailty pool");
            pool.next += 1.0;
            std::mem::take(&mut pool.amount)
        };
        if amount <= 0.0 {
            continue;
        }
        let Some(spell) = spell_cast::load_triggered_spell(world, 227255, 1) else {
            error!(?actor, "native Frailty heal missing");
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
