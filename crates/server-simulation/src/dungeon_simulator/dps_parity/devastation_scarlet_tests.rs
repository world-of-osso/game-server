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
        for _ in 0..40 {
            run.app.update();
        }
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
        for _ in 0..40 {
            run.app.update();
        }
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
    fn periodic(heal: bool) -> f64 {
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
        run.app.insert_resource(
            crate::spell_cast::effects::factor_capture::FactorCapture::new(
                actor,
                target,
                361500,
                "ruby_embers",
                8,
            ),
        );
        let _damage_seed = crate::random::seed_scope(818181);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 361469, Some(target), false)
            .unwrap();
        for _ in 0..300 {
            run.app.update();
        }
        let capture = run
            .app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export();
        capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "periodic_amount" && event["spell_id"] == 361500)
            .expect("actual Ruby Embers periodic damage")["factors"]["amount"]
            .as_f64()
            .unwrap()
    }
    let boosted = periodic(true);
    let plain = periodic(false);
    assert!(
        (boosted / plain - 1.0).abs() < 0.005,
        "SimC bonus_da only affects direct Living Flame, not Ruby Embers: {boosted}/{plain}"
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
        if heal && selected {
            let stored = run
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 372470 && aura.caster == actor)
                .unwrap()
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount;
            assert!(
                stored > 0.0,
                "the visible native aura owns the stored healing"
            );
        }
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
        for _ in 0..40 {
            run.app.update();
        }
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
    let unselected_plain = damage(false, false);
    assert!(
        boosted > plain,
        "actual healing empowers the next damage: {boosted}/{plain}"
    );
    assert!(
        (unselected - unselected_plain).abs() < 0.01,
        "unselected healing grants no damage bonus: {unselected}/{unselected_plain}"
    );
}

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_scarlet_banks_healing_before_the_critical_multiplier() {
    fn sample(crit_pct: f32) -> (f32, bool) {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(1467, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = crit_pct;
        let _cast_seed = crate::random::seed_scope(818181);
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 361509, Some(actor), 1);
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        let event = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(361509)
                    && event.kind == shared::protocol::CombatLogKind::Heal
            })
            .expect("real native heal");
        let critical = event.crit;
        let stored = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 372470 && aura.caster == actor)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount;
        (stored, critical)
    }
    let (ordinary, plain_crit) = sample(-1000.0);
    let (critical, did_crit) = sample(1000.0);
    assert!(
        !plain_crit && did_crit,
        "the actual native heals must exercise both crit paths"
    );
    assert!(
        ordinary > 0.0 && (ordinary - critical).abs() < 0.1,
        "Scarlet must use SimC result_raw, not post-crit healing: {ordinary}/{critical}"
    );
}

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_scarlet_banks_periodic_raw_before_partial_tick_fraction() {
    fn sample(fraction: f32) -> (f32, i32) {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(1467, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = -1000.0;
        let mut spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(361509)
            .unwrap()
            .clone();
        spell.cast_periodic_fraction = Some(fraction);
        let ctx = crate::spell_cast::effects::EffectCtx {
            caster: actor,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        crate::spell_cast::effects::apply_heal(
            run.app.world_mut(),
            &ctx,
            actor,
            1000.0,
            false,
            true,
        );
        let event = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .rev()
            .find(|event| {
                event.spell_id == Some(361509)
                    && event.kind == shared::protocol::CombatLogKind::Heal
            })
            .expect("native periodic heal");
        let healing = event.amount;
        let stored = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 372470 && aura.caster == actor)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount;
        (stored, healing)
    }
    let (full, full_heal) = sample(1.0);
    let (partial, partial_heal) = sample(0.5);
    assert!(
        partial_heal < full_heal,
        "partial tick must really reduce resolved healing"
    );
    assert!(
        full > 0.0 && (full - partial).abs() < 0.1,
        "SimC result_raw precedes partial tick scaling: {full}/{partial}"
    );
}
