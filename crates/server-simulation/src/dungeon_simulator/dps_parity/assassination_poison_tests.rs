//! Native coating procs must produce their sourced damage through real attacks.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_deadly_coating_produces_dot_and_repeat_instant() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 2823));
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..800 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.spell_id == spell
            })
            .map(|event| event.amount)
            .sum::<i32>()
    };
    assert!(
        damage(None) > 0,
        "real weapon swings must reach the coating's proc boundary"
    );
    assert!(
        damage(Some(2818)) > 0,
        "native Deadly Poison must apply and tick from the earned coating procs"
    );
    assert!(
        damage(Some(113780)) > 0,
        "later Deadly procs against the owned ticking poison must also deal instant damage"
    );
}
