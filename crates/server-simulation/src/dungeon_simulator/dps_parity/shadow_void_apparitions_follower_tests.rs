use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::UnitLevel;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class summon templates"]
fn follower_void_apparitions_slam_activates_idols_without_cthun_talent() {
    for selected in [true, false] {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 90);
        assert!(crate::auras::has_aura(app.world(), actor, 1264107));
        assert!(!crate::auras::has_aura(app.world(), actor, 377349));
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::Follower>();
        crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
        if !selected {
            let known = app
                .world()
                .get::<crate::class_progression::KnownSpells>(actor)
                .unwrap()
                .iter()
                .filter(|spell| *spell != 1264107)
                .collect();
            app.world_mut()
                .entity_mut(actor)
                .insert(crate::class_progression::KnownSpells::fixture(known));
            crate::auras::remove_auras(
                app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 1264107,
            );
        }
        let mut shadowfiend_level = None;
        let mut fiend_buff_seen = false;
        for _ in 0..17 {
            crate::spell_cast::cast_triggered(app.world_mut(), actor, 1227280, Some(target), 0);
            for _ in 0..25 {
                app.update();
                let world = app.world_mut();
                let fiends: Vec<_> = world
                    .query::<(&crate::spell_cast::summon::SummonedCreature, &UnitLevel)>()
                    .iter(world)
                    .filter(|(summon, _)| summon.owner == actor && summon.spell_id == 1280172)
                    .map(|(_, level)| level.0)
                    .collect();
                if let Some(level) = fiends.first() {
                    shadowfiend_level = Some(*level);
                    fiend_buff_seen |= crate::auras::has_aura(world, actor, 373316);
                }
            }
        }
        for _ in 0..250 {
            app.update();
        }
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let summons = |spell| {
            events
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::Summon && event.spell_id == Some(spell)
                })
                .count()
        };
        let hits = |spell| {
            events
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::Damage
                        && event.source == Some(actor.to_bits())
                        && event.spell_id == Some(spell)
                })
                .count()
        };
        if selected {
            assert!(
                summons(377355) >= 7,
                "C'Thun outcomes must spawn without talent 377349"
            );
            assert!(summons(373277) >= 2, "Yogg outcomes must spawn Things");
            assert!(
                summons(1280172) >= 3,
                "Y'Shaarj outcomes must spawn Shadowfiends"
            );
            assert!(
                hits(1243105) >= 4 && hits(1243106) >= 1,
                "both N'Zoth outcomes must damage the target"
            );
            assert_eq!(shadowfiend_level, Some(90));
            assert!(
                fiend_buff_seen,
                "earned Shadowfiend must grant Y'Shaarj haste"
            );
            assert!(
                !crate::auras::has_aura(app.world(), actor, 373316),
                "haste expires with the last fiend"
            );
        } else {
            assert_eq!(summons(377355), 0, "no C'Thun without either producer");
            assert_eq!(
                summons(1280172),
                0,
                "Slam must not summon Shadowfiend without 1264107"
            );
        }
    }
}
