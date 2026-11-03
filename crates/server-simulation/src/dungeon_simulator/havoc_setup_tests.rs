//! Native Murder Row regression: an unavailable retreat setup cannot lock Havoc's APL.
use super::*;
use shared::protocol::CombatLogKind;

fn setup() -> (Run, Entity, Entity) {
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
                .is_some_and(|f| f.profile.spec_id == 577)
        })
        .unwrap();
    for _ in 0..2400 {
        tick(&mut run);
        let world = run.app.world();
        if world
            .get::<crate::simc_apl_native::AplMovementIntent>(havoc)
            .is_some()
        {
            let target = world
                .get::<crate::networking::CombatTarget>(havoc)
                .and_then(|target| target.0)
                .unwrap();
            assert!(crate::spell_cast::targets::is_alive(world, target));
            return (run, havoc, target);
        }
    }
    panic!("Havoc never entered the observed retreat setup");
}

fn tick(run: &mut Run) {
    assert!(!run.finished());
    run.control().unwrap();
    run.app.update();
    run.healer_metrics.observe_frame(run.app.world());
    let mut key = run.keystone.take().unwrap();
    key.update(run.app.world_mut()).unwrap();
    run.keystone = Some(key);
}

fn cast_after(run: &Run, actor: Entity, cursor: usize) -> bool {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..]
        .iter()
        .any(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id != Some(198793)
        })
}

#[test]
#[ignore = "requires Murder Row Mythic graph and navmesh"]
fn havoc_keystone_retreat_setup_completes() {
    let _rng = crate::random::seed_scope(7809098381633315392);
    let (mut run, havoc, _) = setup();
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..400 {
        tick(&mut run);
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..];
        if events.iter().any(|event| {
            event.source == Some(havoc.to_bits())
                && event.spell_id == Some(198793)
                && event.kind == CombatLogKind::CastSuccess
        }) {
            return;
        }
    }
    panic!("keystone Havoc never completes its selected Retreat");
}

#[test]
#[ignore = "requires Murder Row Mythic graph and navmesh"]
fn havoc_infeasible_retreat_setup_does_not_stop_other_casts() {
    let _rng = crate::random::seed_scope(7809098381633315392);
    let (mut run, havoc, target) = setup();
    // Keep setup unready rather than relying on a moving target to remain awkward.
    run.app
        .world_mut()
        .get_mut::<crate::creature_ai::SpawnPoint>(havoc)
        .unwrap()
        .speed_run = 0.0;
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..200 {
        tick(&mut run);
        assert!(
            run.app
                .world()
                .get::<server_follower_apl::FollowerAplFailed>(havoc)
                .is_none()
        );
        if cast_after(&run, havoc, cursor) {
            assert!(
                crate::spell_cast::targets::is_alive(run.app.world(), target),
                "casting resumed before the setup target died"
            );
            return;
        }
    }
    panic!("Havoc did not cast another action while retreat setup was unready");
}

#[test]
#[ignore = "requires Murder Row Mythic graph and navmesh"]
fn havoc_resumes_when_retreat_setup_target_dies() {
    let _rng = crate::random::seed_scope(7809098381633315392);
    let (mut run, havoc, target) = setup();
    run.app
        .world_mut()
        .get_mut::<Health>(target)
        .unwrap()
        .current = 0.0;
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..400 {
        tick(&mut run);
        assert!(
            run.app
                .world()
                .get::<server_follower_apl::FollowerAplFailed>(havoc)
                .is_none()
        );
        let damaged_another = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..]
            .iter()
            .any(|event| {
                event.source == Some(havoc.to_bits())
                    && event.target != Some(target.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.spell_id.is_some()
                    && event.amount > 0
            });
        if damaged_another {
            return;
        }
    }
    panic!("Havoc did not damage another enemy after the retreat setup target died");
}
