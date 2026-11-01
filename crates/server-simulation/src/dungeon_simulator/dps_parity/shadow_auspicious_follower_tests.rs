use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_shadow_apparition_impacts_generate_auspicious_insanity() {
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
        assert!(crate::auras::has_aura(app.world(), actor, 155271));
        if !selected {
            crate::auras::remove_auras(
                app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 155271 && aura.caster == actor,
            );
        }
        crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 34914, Some(target), 1);
        for _ in 0..5 {
            crate::spell_cast::cast_triggered(app.world_mut(), actor, 335467, Some(target), 1);
            for _ in 0..40 {
                app.update();
            }
        }
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let apparitions = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && matches!(event.spell_id, Some(413231 | 1264176))
                    && event.amount > 0
            })
            .count();
        let gains: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Energize
                    && event.spell_id == Some(405746)
                    && event.amount > 0
            })
            .map(|event| event.amount)
            .collect();
        assert!(
            apparitions >= 5,
            "real Madness must launch apparitions: {apparitions}"
        );
        assert_eq!(
            !gains.is_empty(),
            selected,
            "only selected Auspicious Spirits grants Insanity"
        );
        assert!(
            gains.iter().all(|&amount| amount == 100),
            "one Insanity per proc: {gains:?}"
        );
    }
}
