use super::*;
use super::follower_dps_diagnostic::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_shadow_maddening_touch_generates_insanity_from_vt_ticks() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    for selected in [true, false] {
        let _seed = crate::random::seed_scope(70448);
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2515).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 70);
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::Follower>();
        for _ in 0..4 {
            app.update();
        }
        assert!(crate::auras::has_aura(app.world(), actor, 391228));
        if !selected {
            crate::auras::remove_auras(
                app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 391228 && aura.caster == actor,
            );
        }
        crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
        for cycle in 0..40 {
            if !selected {
                assert!(!crate::auras::has_aura(app.world(), actor, 391228), "talent returned at cycle {cycle}");
            }
            if cycle % 8 == 0 {
                crate::spell_cast::cast_triggered(app.world_mut(), actor, 34914, Some(target), 1);
            }
            for _ in 0..30 {
                app.update();
            }
        }
        let gains: i32 = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| event.kind == CombatLogKind::Energize && event.spell_id == Some(391232))
            .map(|event| event.amount)
            .sum();
        assert_eq!(
            gains > 0,
            selected,
            "VT ticks grant Insanity only with Maddening Touch"
        );
        assert_eq!(gains % 100, 0, "native talent effect grants whole Insanity");
    }
}

