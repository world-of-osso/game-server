//! Scarlet Adaptation uses actual native Evoker healing and Living Flame damage.
use super::*;

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_scarlet_friendly_living_flame_consumes_the_old_pool() {
    fn damage(preheal: bool) -> f64 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(1467, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        if preheal {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 361509, Some(actor), 1);
        }
        preparation::finish_pending_setup(&mut run).unwrap();
        let _cast_seed = crate::random::seed_scope(91234);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 361469, Some(actor), false)
            .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        run.app.insert_resource(
            crate::spell_cast::effects::factor_capture::FactorCapture::new(
                actor,
                target,
                361500,
                "living_flame",
                1,
            ),
        );
        let _damage_seed = crate::random::seed_scope(818181);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 361469, Some(target), false)
            .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let capture = run
            .app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export();
        capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "result" && event["spell_id"] == 361500)
            .expect("actual damage after the friendly cast")["factors"]["raw_amount"]
            .as_f64()
            .unwrap()
    }
    let previous_pool = damage(true);
    let no_previous_pool = damage(false);
    assert!(
        (previous_pool - no_previous_pool).abs() < 0.01,
        "friendly Living Flame consumes the old pool before its own heal: {previous_pool}/{no_previous_pool}"
    );
}

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_scarlet_does_not_empower_ruby_embers_periodic_damage() {
    fn periodic(heal: bool) -> i32 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(1467, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 365937,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        if heal {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 361509, Some(actor), 1);
        }
        preparation::finish_pending_setup(&mut run).unwrap();
        let _damage_seed = crate::random::seed_scope(818181);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 361469, Some(target), false)
            .unwrap();
        for _ in 0..300 {
            run.app.update();
        }
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.spell_id == Some(361500)
                    && event.periodic
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .expect("actual Ruby Embers periodic damage")
            .amount
    }
    assert_eq!(
        periodic(true),
        periodic(false),
        "SimC bonus_da only affects direct Living Flame, not Ruby Embers"
    );
}

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_scarlet_healing_empowers_and_is_consumed_by_real_living_flame() {
    fn damage(heal: bool, selected: bool) -> f64 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(1467, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), actor, 372469));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 372469,
            );
        }
        if heal {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 361509, Some(actor), 1);
        }
        preparation::finish_pending_setup(&mut run).unwrap();
        assert_eq!(
            crate::auras::has_aura(run.app.world(), actor, 372470),
            heal && selected,
            "only selected Scarlet Adaptation stores actual native healing"
        );
        run.app.insert_resource(
            crate::spell_cast::effects::factor_capture::FactorCapture::new(
                actor,
                target,
                361500,
                "living_flame",
                1,
            ),
        );
        let _damage_seed = crate::random::seed_scope(818181);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 361469, Some(target), false)
            .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 372470),
            "damage cast consumes Scarlet"
        );
        let capture = run
            .app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export();
        let event = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "result" && event["spell_id"] == 361500)
            .expect("real native Living Flame damage");
        event["factors"]["raw_amount"].as_f64().unwrap()
    }
    let plain = damage(false, true);
    let boosted = damage(true, true);
    let unselected = damage(true, false);
    assert!(
        boosted > plain,
        "actual healing empowers the next damage: {boosted}/{plain}"
    );
    assert!(
        (unselected - plain).abs() < 0.01,
        "unselected healing grants no damage bonus"
    );
}
