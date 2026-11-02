use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::{self, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_nzoth_thresholds_spawn_apparitions_only_with_apex() {
    for (stacks, idol_damage) in [(49, 1243105), (99, 1243106)] {
        for apex in [false, true] {
            let _seed = crate::random::seed_scope(70448);
            let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
            let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
            let copy = create_open_follower_copy(&mut app, 2813).unwrap();
            let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 90);
            app.world_mut()
                .entity_mut(actor)
                .remove::<crate::followers::Follower>();
            assert!(auras::has_aura(app.world(), actor, 373280));
            assert!(auras::has_aura(app.world(), actor, 1264096));
            if !apex {
                auras::remove_auras(app.world_mut(), actor, RemoveReason::Cancelled, |aura| {
                    aura.spell_id == 1264096
                });
            }
            for spell_id in [34914, 1243069] {
                auras::apply_aura(
                    app.world_mut(),
                    &AuraApplication {
                        caster: actor,
                        target,
                        spell_id,
                        effect_mask: u32::MAX,
                        source: AuraSource::Cast,
                    },
                );
            }
            auras::set_owned_stacks(app.world_mut(), actor, target, 1243069, stacks);
            app.world_mut()
                .resource_mut::<ObservedCombatEvents>()
                .0
                .clear();
            crate::spell_cast::cast_triggered(app.world_mut(), actor, 8092, Some(target), 0);
            for _ in 0..40 {
                app.update();
            }
            let events = &app.world().resource::<ObservedCombatEvents>().0;
            let landed = |spell| {
                events
                    .iter()
                    .filter(|event| {
                        event.kind == CombatLogKind::Damage
                            && event.spell_id == Some(spell)
                            && event.source == Some(actor.to_bits())
                            && event.target == Some(target.to_bits())
                    })
                    .count()
            };
            assert_eq!(landed(idol_damage), 1, "threshold={stacks} apex={apex}");
            assert_eq!(
                landed(413231) + landed(1264176),
                usize::from(apex),
                "threshold={stacks} apex={apex}"
            );
        }
    }
}
