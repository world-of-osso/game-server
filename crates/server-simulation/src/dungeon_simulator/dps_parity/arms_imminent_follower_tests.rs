use super::*;
use super::follower_dps_diagnostic::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_imminent_demise_adds_bladestorm_ticks_and_expires() {
    fn spin(stacks: usize) -> usize {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 90);
        for _ in 0..5 {
            app.update();
        }
        assert!(crate::auras::has_aura(app.world(), actor, 444769));
        assert!(crate::auras::has_aura(app.world(), actor, 29725));
        for _ in 0..stacks {
            crate::spell_cast::cast_triggered(app.world_mut(), actor, 445606, Some(actor), 1);
        }
        let spin =
            crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 227847).unwrap();
        assert_eq!(
            spin, 446035,
            "the selected native Slayer action must resolve"
        );
        crate::spell_cast::creature_cast(app.world_mut(), actor, spin, Some(actor), false).unwrap();
        assert!(
            crate::auras::has_aura(app.world(), actor, 446035),
            "the native spin must start"
        );
        for _ in 0..500 {
            app.update();
            if !crate::auras::has_aura(app.world(), actor, 446035) {
                break;
            }
        }
        assert!(!crate::auras::has_aura(app.world(), actor, 446035));
        assert!(
            !crate::auras::has_aura(app.world(), actor, 445606),
            "spin completion consumes earned Imminent Demise"
        );
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(50622)
                    && event.amount > 0
            })
            .count()
    }
    let ordinary = spin(0);
    let enhanced = spin(3);
    assert_eq!(
        enhanced,
        ordinary + 3,
        "three Imminent Demise stacks add three actual ticks"
    );
}
