//! A real Magus's Withering Grasp outlives its conversion into a Lord.
use super::*;
use crate::spell_cast::{summon::SummonedCreature, test_harness::ObservedCombatEvents};
use shared::{components::Health, protocol::CombatLogKind};

#[test]
#[ignore = "requires native252 profile and real Magus/Lord summons"]
fn unholy_grasp_ticks_after_magus_is_consumed() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    let owner = run.actor;
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        owner,
        literal_rotation::execution::Execution::Spell(42650, owner, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let magus = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &SummonedCreature)>()
            .iter(world)
            .find_map(|(entity, summon)| {
                (summon.owner == owner && summon.spell_id == 317776).then_some(entity)
            })
            .expect("actual Army Magus")
    };
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        magus,
        1297091,
        Some(target),
        1
    ));
    for _ in 0..2 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            owner,
            1242294,
            Some(owner),
            1
        ));
    }
    for _ in 0..30 {
        run.app.update();
        if run
            .app
            .world()
            .get::<Health>(magus)
            .is_some_and(|health| health.current == 0.0)
        {
            break;
        }
    }
    assert!(
        run.app
            .world()
            .get::<Health>(magus)
            .is_some_and(|health| health.current == 0.0),
        "real Magi must be consumed and retired when a Lord forms"
    );
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..80 {
        run.app.update();
    }
    let ticks = run.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .filter(|event| {
            event.source == Some(magus.to_bits())
                && event.spell_id == Some(1297091)
                && event.periodic
                && event.kind == CombatLogKind::Damage
        })
        .count();
    assert!(
        ticks > 0,
        "the consumed Magus's actual Grasp must keep ticking"
    );
    assert!(
        run.app.world().get_entity(magus).is_err(),
        "retired Magus must be removed when Grasp ends"
    );
}
