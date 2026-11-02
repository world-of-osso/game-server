use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with C'Thun guardian templates"]
fn follower_shadow_cthun_summons_channeling_guardians_and_earns_insanity() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    for (count, selected) in [(1, true), (3, true), (1, false)] {
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
        assert!(crate::auras::has_aura(app.world(), actor, 377349));
        if !selected {
            crate::auras::remove_auras(
                app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 377349 && aura.caster == actor,
            );
        }
        let mut targets = vec![target];
        for i in 1..count {
            targets.push(spawn_level_dummy(&mut app, actor, 70, i).unwrap());
        }
        app.world_mut()
            .entity_mut(actor)
            .insert(crate::networking::CombatTarget(Some(target)));
        for _ in 0..32 {
            for &enemy in &targets {
                let mut health = app
                    .world_mut()
                    .get_mut::<shared::components::Health>(enemy)
                    .unwrap();
                health.current = health.max;
                crate::combat_refs::set_in_combat_with(app.world_mut(), actor, enemy);
            }
            crate::spell_cast::powers::change_power(
                app.world_mut(),
                actor,
                PowerType::Insanity,
                -10000.0,
            );
            crate::spell_cast::creature_cast(app.world_mut(), actor, 15407, Some(target), false)
                .unwrap();
            for _ in 0..100 {
                app.update();
            }
        }
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let tendril = events.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(193473)
                && event.amount > 0
        });
        let lasher = events.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(394979)
                && event.amount > 0
        });
        let gains: Vec<_> = events
            .iter()
            .filter(|event| {
                event.target == Some(actor.to_bits())
                    && event.kind == CombatLogKind::Energize
                    && event.spell_id == Some(377358)
                    && event.amount > 0
            })
            .map(|event| event.amount)
            .collect();
        if count == 1 {
            assert_eq!(
                tendril, selected,
                "single-target C'Thun summons a damaging Tendril"
            );
        }
        assert_eq!(
            lasher,
            selected && count == 3,
            "three engaged targets allow a damaging Lasher"
        );
        assert_eq!(
            !gains.is_empty(),
            selected,
            "actual guardian ticks grant Insanity only with C'Thun"
        );
        assert!(
            gains.iter().all(|&amount| amount == 100),
            "each pet tick grants one Insanity: {gains:?}"
        );
    }
}
