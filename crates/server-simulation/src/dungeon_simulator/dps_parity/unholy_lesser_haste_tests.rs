//! A real lesser ghoul uses its owner's live attack haste.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn two_swings(run: &mut Run, ghoul: Entity) -> f64 {
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let mut times = Vec::new();
    for _ in 0..200 {
        run.app.update();
        let now = run
            .app
            .world()
            .resource::<Time<Virtual>>()
            .elapsed_secs_f64();
        let mut events = run.app.world_mut().resource_mut::<ObservedCombatEvents>();
        for event in events.0.drain(..) {
            if event.source == Some(ghoul.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
            {
                times.push(now);
            }
        }
        if times.len() >= 2 {
            return times[1] - times[0];
        }
    }
    panic!("real lesser ghoul did not swing twice");
}

#[test]
#[ignore = "requires native252 profile and lesser ghoul template"]
fn unholy_lesser_ghoul_swings_follow_live_owner_haste() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    engage(&mut run);
    let owner = run.actor;
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        owner,
        275430,
        Some(owner),
        1
    ));
    let ghoul = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &SummonedCreature)>()
            .iter(world)
            .find_map(|(entity, summon)| {
                (summon.owner == owner && summon.spell_id == 275430).then_some(entity)
            })
            .expect("real lesser ghoul")
    };
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .haste_pct = 0.0;
    let baseline = two_swings(&mut run, ghoul);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .haste_pct = 100.0;
    // The currently pending swing retains its deadline; the following interval uses live haste.
    let fast = two_swings(&mut run, ghoul);
    assert!(
        (fast * 2.0 - baseline).abs() <= 0.1,
        "real ghoul swing interval must halve when owner haste doubles: {baseline} -> {fast}"
    );
}
