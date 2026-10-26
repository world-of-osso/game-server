//! Actual Seed casts, threshold/expiry and native explosion→owned Wither.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const SEED: u32 = 27243;
const EXPLOSION: u32 = 27285;
const WITHER: u32 = 445474;

fn cast_native_action(run: &mut Run, requested: u32, target: Entity) -> u32 {
    let actual = literal_rotation::execution::resolve_action(run.app.world(), run.actor, requested)
        .unwrap()
        .expect("native selected Seed fixture action");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
    actual
}

fn seed_expiry(run: &Run) -> Option<f64> {
    run.app
        .world()
        .get::<ActiveAuras>(run.targets[0])?
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == SEED)?
        .expires_at
}

fn prepare_actual_seed(count: usize) -> (Run, f64, f32) {
    let mut run = prepare_literal(265, count).unwrap();
    // Profile starts at0 shards. Existing ordinary pet setup waits for native
    // OOC readiness and pays its real cost; no pool reset/grant is introduced.
    preparation::start_configured_native_pet(&mut run).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    cast_native_action(&mut run, SEED, target);
    for _ in 0..100 {
        if let Some(expiry) = seed_expiry(&run) {
            let threshold = read_seed_threshold(&run);
            return (run, expiry, threshold);
        }
        run.app.update();
    }
    panic!("ordinary paid27243 must reach its actual native aura");
}

fn read_seed_threshold(run: &Run) -> f32 {
    // SimC76f8118 actions1931: threshold snapshots real caster SP×s1%.
    let world = run.app.world();
    let power = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .spell_power;
    let percent = world
        .resource::<crate::spell_info::SpellStore>()
        .get(SEED)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    power * percent / 100.0
}

fn assert_actual_explosion_and_wither(run: &Run) {
    let world = run.app.world();
    for &target in &run.targets {
        let expected = (
            Some(run.actor.to_bits()),
            Some(target.to_bits()),
            Some(EXPLOSION),
        );
        let landed = world
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                let identity = (event.source, event.target, event.spell_id);
                let damage = event.kind == CombatLogKind::Damage;
                identity == expected && damage && event.amount > 0
            });
        assert!(
            landed,
            "actual Seed must emit native27285 at real pack recipient{target:?}"
        );
        assert!(
            world.get::<ActiveAuras>(target).is_some_and(|auras| auras
                .iter()
                .any(|aura| { aura.caster == run.actor && aura.spell_id == WITHER })),
            "actual27285 must apply owned native Wither at{target:?}"
        );
    }
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn seed_native_natural_expiry_explodes_and_applies_owned_wither_to_pack() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, expiry, _) = prepare_actual_seed(3);
    // No player rotation or pet attack order after the paid Seed: let the
    // actual native aura expire rather than granting/removing it in the fixture.
    for _ in 0..600 {
        if crate::spell_cast::now(run.app.world()) > expiry + 0.25 {
            break;
        }
        run.app.update();
    }
    assert!(seed_expiry(&run).is_none(), "native Seed must actually end");
    assert_actual_explosion_and_wither(&run);
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn seed_native_refresh_keeps_original_explosion_timer_without_resource_reset() {
    let _seed = crate::random::seed_scope(70448);
    // One real enemy forces the source's original-target reapplication case.
    let (mut run, original_expiry, _) = prepare_actual_seed(1);
    for _ in 0..20 {
        run.app.update();
    }
    let target = run.targets[0];
    cast_native_action(&mut run, SEED, target);
    let mut refreshed = original_expiry;
    for _ in 0..20 {
        run.app.update();
        refreshed = seed_expiry(&run).expect("actual paid reapplication keeps native Seed");
        if refreshed > original_expiry {
            break;
        }
    }
    assert!(
        refreshed > original_expiry,
        "actual reapplication must reach its extended native aura"
    );
    // actions1937–1965: refresh extends the aura but cannot move explosion's
    // original tick. This reproduces the literal APL's repeated Seed ordering.
    for _ in 0..600 {
        if crate::spell_cast::now(run.app.world()) > original_expiry + 0.25 {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::spell_cast::now(run.app.world()) < refreshed,
        "do not borrow refreshed natural expiry"
    );
    assert_actual_explosion_and_wither(&run);
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn seed_native_actual_owner_damage_threshold_explodes_before_expiry_and_applies_wither() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, expiry, threshold) = prepare_actual_seed(3);
    let target = run.targets[0];
    let actual = cast_native_action(&mut run, 686, target);
    let mut reached = false;
    for _ in 0..100 {
        run.app.update();
        reached = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                let identity = (event.source, event.target, event.spell_id);
                let expected = (
                    Some(run.actor.to_bits()),
                    Some(target.to_bits()),
                    Some(actual),
                );
                let damage = event.kind == CombatLogKind::Damage;
                identity == expected && damage && event.amount as f32 >= threshold
            });
        if reached {
            break;
        }
    }
    assert!(
        reached,
        "real owner Shadow Bolt must cross native Seed threshold{threshold}"
    );
    // actions380–385/1937–1965 queues the explosion after threshold removal.
    for _ in 0..5 {
        run.app.update();
    }
    assert!(
        crate::spell_cast::now(run.app.world()) < expiry,
        "threshold case must not borrow natural expiry"
    );
    assert_actual_explosion_and_wither(&run);
}
