//! Fixed-fight followers model SimC's Karma absorption; ordinary followers do not.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_karma_models_absorption_only_in_fixed_fight() {
    for modeled in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2515).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 269, 70);
        if modeled {
            app.world_mut()
                .entity_mut(actor)
                .insert(server_follower_apl::windwalker_rotation::karma::ModeledKarmaFixture);
        }
        let owner = app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .owner;
        for _ in 0..5 {
            app.update();
        }
        start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
        for frame in 0..FIXED_FIGHT_FRAMES {
            let mut health = app
                .world_mut()
                .get_mut::<shared::components::Health>(target)
                .unwrap();
            health.current =
                health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
            app.update();
        }
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let casts = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.kind == CombatLogKind::CastSuccess
                    && event.spell_id == Some(122470)
            })
            .count();
        let ticks: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(124280)
            })
            .collect();
        assert_eq!(casts, 2, "real follower must cast Karma twice");
        if modeled {
            assert_eq!(
                ticks.len(),
                12,
                "two modeled redirects each deliver six ticks"
            );
            assert!(ticks.iter().all(|event| event.amount > 0));
        } else {
            assert!(
                ticks.is_empty(),
                "ordinary followers do not get simulated incoming damage"
            );
        }
    }
}
