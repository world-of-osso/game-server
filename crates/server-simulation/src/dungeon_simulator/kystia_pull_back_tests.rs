//! Murder Row: an engaged boss must not leave Arms waiting for pull-back.
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires Murder Row Mythic graph and native navigation"]
fn arms_attacks_engaged_kystia_during_pull_back() {
    let seed = 7809098381633315392;
    let _rng_scope = crate::random::seed_scope(seed);
    let mut run = spawn_run_mode(
        seed,
        &mut None,
        Some(super::super::keystone::Options {
            map_id: 2813,
            level: 2,
            specs: [581, 264, 71, 577, 251],
        }),
    )
    .unwrap();
    run.last_progress = run.now();
    let arms = *run
        .party
        .members
        .iter()
        .find(|&&unit| {
            run.app
                .world()
                .get::<crate::followers::Follower>(unit)
                .is_some_and(|follower| follower.profile.spec_id == 71)
        })
        .unwrap();
    let boss = *run
        .actors
        .values()
        .find(|&&unit| {
            run.app
                .world()
                .get::<shared::components::Npc>(unit)
                .is_some_and(|npc| npc.template_id == 234648)
        })
        .unwrap();
    let mut prepared = None;
    for _ in 0..MAX_TICKS {
        assert!(!run.finished(), "keystone ended before Arms engaged Kystia");
        assert!(
            run.now() < 300.0,
            "party did not engage Kystia within 300 simulated seconds"
        );
        run.control().unwrap();
        run.app.update();
        run.healer_metrics.observe_frame(run.app.world());
        let mut key = run.keystone.take().unwrap();
        key.update(run.app.world_mut()).unwrap();
        run.keystone = Some(key);
        let engaged = run
            .app
            .world()
            .get::<crate::networking::CombatTarget>(boss)
            .is_some_and(|target| target.0 == Some(run.party.tank));
        if prepared.is_none() && engaged {
            let world = run.app.world_mut();
            let position = *world.get::<Position>(boss).unwrap();
            world.entity_mut(arms).insert(Position {
                x: position.x + 4.0,
                ..position
            });
            let mut pools = world
                .get_mut::<crate::spell_cast::powers::PowerPools>(arms)
                .unwrap();
            let rage = pools.get_mut(PowerType::Rage).unwrap();
            rage.current = rage.max;
            crate::combat_refs::set_in_combat_with(world, boss, arms);
            crate::followers::set_test_pull_back(world, run.party.owner, run.party.tank);
            assert!(crate::followers::holding_pull_damage(world, arms));
            let cursor = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            prepared = Some((run.now(), cursor));
            continue;
        }
        if let Some((start, cursor)) = prepared {
            let world = run.app.world();
            assert!(
                !crate::followers::holding_pull_damage(world, arms),
                "Arms remains held after Kystia engaged the tank"
            );
            let events = &world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            if events[cursor..].iter().any(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(arms.to_bits())
                    && event.target == Some(boss.to_bits())
                    && event.spell_id.is_some()
                    && event.amount > 0
            }) {
                return;
            }
            assert!(
                run.now() - start <= 5.0,
                "Arms did not damage Kystia within five seconds beside the engaged boss"
            );
        }
    }
    panic!("party never engaged Kystia");
}
