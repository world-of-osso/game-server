use super::marksmanship_mechanics_tests::{cast, player};
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Marksmanship profile and world data"]
fn marksmanship_chakram_damage_increases_on_every_bounce() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = player(1);
    cast(&mut run, 288613, true);
    let actor = run.actor;
    let target = run.targets[0];
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1253601,
    );
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .crit_pct = 0.0;
    cast(&mut run, 1264949, false);
    for _ in 0..150 {
        run.app.update();
    }
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(1266081)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(hits.len(), 7, "a single Chakram makes seven impacts");
    assert!(hits.iter().all(|hit| !hit.crit && hit.amount > 0));
    let ratio = f64::from(hits[6].amount) / f64::from(hits[0].amount);
    assert!(
        (3.2..4.6).contains(&ratio),
        "seventh hit should reflect six 25% increases, got {ratio}: {hits:?}"
    );
}
