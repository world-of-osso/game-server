use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::{self, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::{summon::SummonedCreature, test_harness::ObservedCombatEvents};
use shared::components::{Level, PowerType, UnitLevel};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with native Voidwraith"]
fn follower_rift_end_spawns_a_damaging_insanity_generating_voidwraith() {
    for selected in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 90);
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::Follower>();
        assert!(auras::has_aura(app.world(), actor, 451234));
        if !selected {
            auras::remove_auras(app.world_mut(), actor, RemoveReason::Cancelled, |aura| {
                aura.spell_id == 451234
            });
        }
        crate::spell_cast::powers::change_power(
            app.world_mut(),
            actor,
            PowerType::Insanity,
            -10000.0,
        );
        auras::apply_aura(
            app.world_mut(),
            &AuraApplication {
                caster: actor,
                target,
                spell_id: 263165,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        assert!(
            auras::has_aura(app.world(), actor, 459314),
            "Torrent must start a real Rift"
        );
        auras::remove_auras(app.world_mut(), actor, RemoveReason::Expired, |aura| {
            aura.spell_id == 459314
        });
        let pet = app
            .world_mut()
            .query::<(bevy::prelude::Entity, &SummonedCreature)>()
            .iter(app.world())
            .find_map(|(entity, summon)| {
                (summon.owner == actor && summon.spell_id == 451235).then_some(entity)
            });
        assert_eq!(pet.is_some(), selected);
        if let Some(pet) = pet {
            assert_eq!(
                app.world().get::<UnitLevel>(pet).unwrap().0,
                app.world().get::<Level>(actor).unwrap().0
            );
            for _ in 0..80 {
                app.update();
            }
            let events = &app.world().resource::<ObservedCombatEvents>().0;
            assert!(
                events
                    .iter()
                    .any(|event| event.source == Some(pet.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.kind == CombatLogKind::Damage
                        && event.spell_id == Some(451435)
                        && event.amount > 0),
                "Voidwraith must damage the actual target"
            );
            assert!(
                events
                    .iter()
                    .any(|event| event.source == Some(actor.to_bits())
                        && event.kind == CombatLogKind::Energize
                        && event.spell_id == Some(262485)
                        && event.amount > 0),
                "Void Flay must grant its owner Insanity"
            );
        }
    }
}
