//! Annihilator Catastrophe rolls meteor damage into a native periodic aura.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

const CATASTROPHE: u32 = 1253769;
struct PendingDeposit {
    target: Entity,
    spell: u32,
    amount: f32,
    at: f64,
}
#[derive(Component, Default)]
struct PendingDeposits(Vec<PendingDeposit>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(impact),
    SpellScriptHook::AfterNativeCastUpdate(update),
];

fn impact(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, damage: f32) {
    if damage <= 0.0
        || !world
            .get::<combat::class_progression::TraitLoadout>(ctx.caster)
            .is_some_and(|traits| {
                traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == CATASTROPHE)
            })
    {
        return;
    }
    let dot = match world.get::<ActiveSpec>(ctx.caster).map(|spec| spec.0) {
        Some(581) if matches!(ctx.spell.id, 1256306 | 1256617 | 1264130) => 1256667,
        Some(1480) if matches!(ctx.spell.id, 1256305 | 1256619 | 1264129) => 1256676,
        _ => return,
    };
    let percent = world
        .resource::<SpellStore>()
        .get(CATASTROPHE)
        .and_then(|s| s.effects.iter().find(|e| e.index == 0))
        .expect("native Catastrophe percentage")
        .base_points;
    assert!(percent >= 0.0, "invalid native Catastrophe percentage");
    // SimC76f8118 sc_demon_hunter.cpp3115–3137 and player.cpp900–923:
    // each landed meteor deposits after the normal 30ms±5ms aura delay.
    let radial = 1.0 - combat::random::random::<f64>();
    let angle = std::f64::consts::TAU * combat::random::random::<f64>();
    let delay = (0.030 + 0.005 * (-2.0 * radial.ln()).sqrt() * angle.cos()).max(0.0);
    let at = combat::spell_cast::now(world) + delay;
    world
        .entity_mut(ctx.caster)
        .entry::<PendingDeposits>()
        .or_default()
        .into_mut()
        .0
        .push(PendingDeposit {
            target,
            spell: dot,
            amount: damage * percent / 100.0,
            at,
        });
}

fn update(world: &mut World) {
    let now = combat::spell_cast::now(world);
    let actors: Vec<_> = world
        .query_filtered::<Entity, With<PendingDeposits>>()
        .iter(world)
        .collect();
    for caster in actors {
        let due = {
            let mut pending = world.get_mut::<PendingDeposits>(caster).unwrap();
            let (due, later) = std::mem::take(&mut pending.0)
                .into_iter()
                .partition(|item| item.at <= now);
            pending.0 = later;
            due
        };
        for deposit in due {
            apply_deposit(world, caster, deposit);
        }
    }
}
fn apply_deposit(world: &mut World, caster: Entity, deposit: PendingDeposit) {
    if !combat::spell_cast::targets::is_alive(world, deposit.target) {
        return;
    }
    let spell = world
        .resource::<SpellStore>()
        .get(deposit.spell)
        .expect("native Catastrophe periodic spell")
        .clone();
    let duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("native Catastrophe duration")
        .duration_ms;
    assert!(duration > 0, "invalid native Catastrophe duration");
    combat::auras::apply_rolling_periodic_deposit(
        world,
        &combat::auras::AuraApplication {
            caster,
            target: deposit.target,
            spell_id: deposit.spell,
            effect_mask: 1,
            source: combat::auras::AuraSource::Cast,
        },
        &spell,
        duration as u32,
        deposit.amount,
    );
}
