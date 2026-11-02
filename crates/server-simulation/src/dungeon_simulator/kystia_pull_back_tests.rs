//! Murder Row: an engaged boss must not leave Arms waiting for pull-back.
use super::*;
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
    let mut close_at = None;
    let mut cursor = 0;
    for _ in 0..MAX_TICKS {
        assert!(!run.finished(), "keystone ended before Arms engaged Kystia");
        run.control().unwrap();
        run.app.update();
        run.healer_metrics.observe_frame(run.app.world());
        let mut key = run.keystone.take().unwrap();
        key.update(run.app.world_mut()).unwrap();
        run.keystone = Some(key);
        let world = run.app.world();
        let engaged = world
            .get::<crate::networking::CombatTarget>(boss)
            .is_some_and(|target| target.0 == Some(run.party.tank));
        let close = world
            .get::<Position>(arms)
            .zip(world.get::<Position>(boss))
            .is_some_and(|(a, b)| {
                Vec3::new(a.x, a.y, a.z).distance(Vec3::new(b.x, b.y, b.z)) <= 6.0
            });
        if close_at.is_none() && engaged && close {
            close_at = Some(run.now());
            cursor = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
        }
        if let Some(start) = close_at {
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
    panic!("Arms never reached the engaged boss");
}
