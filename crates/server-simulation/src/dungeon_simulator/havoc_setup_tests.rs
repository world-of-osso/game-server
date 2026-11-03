//! Native Murder Row regression for Havoc's Retreat movement.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires Murder Row Mythic graph and navmesh"]
fn havoc_keystone_retreat_casts_without_edge_setup() {
    let _rng = crate::random::seed_scope(7809098381633315392);
    let mut run = spawn_run_mode(
        7809098381633315392,
        &mut None,
        Some(super::super::keystone::Options {
            map_id: 2813,
            level: 2,
            specs: [581, 264, 71, 577, 251],
        }),
    )
    .unwrap();
    run.last_progress = run.now();
    let havoc = *run
        .party
        .members
        .iter()
        .find(|&&unit| {
            run.app
                .world()
                .get::<crate::followers::Follower>(unit)
                .is_some_and(|follower| follower.profile.spec_id == 577)
        })
        .unwrap();
    for _ in 0..2400 {
        assert!(!run.finished());
        run.control().unwrap();
        run.app.update();
        run.healer_metrics.observe_frame(run.app.world());
        let mut key = run.keystone.take().unwrap();
        key.update(run.app.world_mut()).unwrap();
        run.keystone = Some(key);
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if events.iter().any(|event| {
            event.source == Some(havoc.to_bits())
                && event.spell_id == Some(198793)
                && event.kind == CombatLogKind::CastSuccess
        }) {
            return;
        }
    }
    panic!("keystone Havoc never casts its admitted Retreat");
}
