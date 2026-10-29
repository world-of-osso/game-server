//! Exploding Keg grants the five-hit Empty the Cellar action and Brew recovery.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn cellar_spends_earned_buff_and_reduces_brew_recharge_per_hit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(target), false)
        .unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(target), false)
        .unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 325153, Some(target), false)
        .unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 1262768));
    for _ in 0..30 {
        run.app.update();
    }
    let now = crate::spell_cast::now(run.app.world());
    let history = run
        .app
        .world()
        .get::<crate::spell_cast::SpellHistory>(actor)
        .unwrap();
    let (used_before, before) = crate::spell_cast::test_harness::charge_recovery(history, 1562);
    assert_eq!(used_before, 2);
    let stats = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap();
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(119582)
        .unwrap();
    let duration = f64::from(spell.categories.as_ref().unwrap().charge_recovery_ms)
        / 1000.0
        / (1.0 + f64::from(stats.haste_pct) / 100.0);
    let remaining_before = before.unwrap() - now + duration;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1263438, Some(target), false)
        .unwrap();
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1262768));
    for _ in 0..30 {
        run.app.update();
    }
    let world = run.app.world();
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(1262765)
                && event.kind == CombatLogKind::Damage
        })
        .count();
    assert_eq!(hits, 5, "five actual Cellar impacts");
    let (used_after, after) = crate::spell_cast::test_harness::charge_recovery(
        world.get::<crate::spell_cast::SpellHistory>(actor).unwrap(),
        1562,
    );
    let remaining_after =
        after.unwrap() - crate::spell_cast::now(world) + duration * f64::from(used_after - 1);
    let elapsed = crate::spell_cast::now(world) - now;
    assert!(
        (remaining_before - remaining_after - elapsed - 15.0).abs() < 0.1,
        "each of five impacts removes three seconds: before={remaining_before} after={remaining_after} elapsed={elapsed}"
    );
}
