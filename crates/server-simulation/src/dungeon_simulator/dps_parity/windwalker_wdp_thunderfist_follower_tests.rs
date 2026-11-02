//! The first real WDP area pulse earns Thunderfist once per cast.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_wdp_first_pulse_grants_thunderfist_per_target() {
    for target_count in [1, 3] {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2515).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 269, 70);
        assert!(crate::auras::has_aura(app.world(), actor, 392985));
        for index in 1..target_count {
            spawn_level_dummy(&mut app, actor, 70, index).unwrap();
        }
        // Disable the rotation so no Windlord cast or white swing changes the stack count.
        app.world_mut()
            .entity_mut(actor)
            .take::<crate::followers::Follower>()
            .unwrap();
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 152175, Some(target), 1);
        app.update();
        let stacks = |app: &App| {
            app.world()
                .get::<crate::auras::ActiveAuras>(actor)
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|a| a.spell_id == 393565 && a.caster == actor)
                })
                .map_or(0, |a| a.stacks)
        };
        let expected = 4 + target_count as u8 - 1;
        let hits = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(158221)
            })
            .count();
        assert_eq!(
            hits, target_count,
            "first native area pulse must hit all fixture targets"
        );
        assert_eq!(
            stacks(&app),
            expected,
            "first WDP pulse earns native stacks"
        );
        for _ in 0..24 {
            app.update();
        }
        assert_eq!(
            stacks(&app),
            expected,
            "later WDP pulses must not grant stacks again"
        );
    }
}
