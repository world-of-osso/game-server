//! SimC76f8118 sc_druid.cpp3474–3515/8116–8149: two independent DoT drivers.
use bevy::prelude::*;
use shared::components::CombatStatus;

use super::combat;
use combat::auras::ActiveAuras;
use combat::random::seq::SliceRandom;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const TALENT: u32 = 202342;
const DAMAGE: u32 = 202497;
const SOLSTICE: u32 = 343648;
const DOTS: [u32; 2] = [164812, 164815];

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterNativeCastUpdate(update_drivers)];

#[derive(Component)]
struct ShootingStarsTimers([f64; 2]);

fn update_drivers(world: &mut World) {
    let now = spell_cast::now(world);
    let actors: Vec<_> = world
        .query::<(
            Entity,
            &ActiveAuras,
            Option<&CombatStatus>,
            Option<&ShootingStarsTimers>,
        )>()
        .iter(world)
        .filter_map(|(actor, auras, combat, timers)| {
            let talent = auras
                .iter()
                .find(|aura| aura.spell_id == TALENT && aura.caster == actor);
            if talent.is_none() || !combat.is_some_and(|combat| combat.0) {
                return timers.map(|_| (actor, None, None));
            }
            let talent = talent.expect("selected shooting stars aura");
            let effect = talent
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("shooting stars native periodic chance");
            let period = f64::from(effect.period_ms) / 1000.0;
            assert!(
                period > 0.0,
                "shooting stars native period must be positive"
            );
            Some((
                actor,
                Some((effect.amount / 100.0, period)),
                timers.map(|timer| timer.0),
            ))
        })
        .collect();
    for (actor, config, previous) in actors {
        let Some((chance, period)) = config else {
            world.entity_mut(actor).remove::<ShootingStarsTimers>();
            continue;
        };
        let Some(mut timers) = previous else {
            world.entity_mut(actor).insert(ShootingStarsTimers([
                now + combat::random::random::<f64>() * period,
                now + combat::random::random::<f64>() * period,
            ]));
            continue;
        };
        for (index, dot) in DOTS.into_iter().enumerate() {
            if now >= timers[index] {
                trigger_stars(world, actor, dot, chance);
                timers[index] += period;
            }
        }
        world.entity_mut(actor).insert(ShootingStarsTimers(timers));
    }
}

fn trigger_stars(world: &mut World, caster: Entity, dot: u32, chance: f32) {
    let now = spell_cast::now(world);
    let map = world.get::<combat::maps::InMap>(caster).copied();
    let mut targets: Vec<_> = world
        .query::<(Entity, &ActiveAuras, &combat::maps::InMap)>()
        .iter(world)
        .filter(|(target, auras, target_map)| {
            Some(**target_map) == map
                && combat::spell_cast::targets::is_alive(world, *target)
                && auras.iter().any(|aura| {
                    aura.spell_id == dot
                        && aura.caster == caster
                        && aura.expires_at.is_none_or(|expiry| expiry > now)
                })
        })
        .map(|(target, _, _)| target)
        .collect();
    if targets.is_empty() {
        return;
    }
    let solstice = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SOLSTICE && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount / 100.0);
    let expected = chance * (1.0 + solstice) * (targets.len() as f32).sqrt();
    let whole = expected.floor() as usize;
    let procs = (whole + usize::from(combat::random::random::<f32>() < expected.fract()))
        .min(targets.len());
    if targets.len() > 1 {
        targets.shuffle(&mut combat::random::thread_rng());
    }
    for target in targets.into_iter().take(procs) {
        spell_cast::cast_triggered(world, caster, DAMAGE, Some(target), 1);
    }
}
