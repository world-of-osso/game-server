//! Incarnation must enter Bear Form from the real Cat Form opener.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn druid_foreground_spell_resets_swing_but_mangle_and_form_do_not() {
    use shared::protocol::CombatLogKind;
    for (spell, should_reset) in [(8921, true), (33917, false), (768, false)] {
        let mut run = prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
        for _ in 0..40 { run.app.update(); }
        run.app.world_mut().entity_mut(actor).insert(crate::combat::AutoAttack::new(target));
        for _ in 0..21 { run.app.update(); }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(if spell == 768 { actor } else { target }), false).unwrap();
        run.app.world_mut().resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.clear();
        for _ in 0..35 { run.app.update(); }
        let swings = run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter()
            .filter(|event| event.source == Some(actor.to_bits()) && event.target == Some(target.to_bits()) && event.spell_id.is_none() && event.kind == CombatLogKind::Damage).count();
        assert_eq!(swings == 0, should_reset, "spell {spell} swings={swings}");
    }
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_bear_passive_changes_stamina_and_swing_rage_only_in_bear() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let stats = |run: &Run| {
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .primary
            .stamina
    };
    let rage_mod = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::AuraStatMods>(actor)
            .unwrap()
            .rage_from_damage_dealt_pct
    };
    let baseline_stamina = stats(&run);
    let baseline_rage_mod = rage_mod(&run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        stats(&run) > baseline_stamina,
        "Bear Form must increase stamina"
    );
    assert_eq!(
        rage_mod(&run),
        baseline_rage_mod - 8.0,
        "Bear passive must reduce swing Rage"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        rage_mod(&run),
        baseline_rage_mod,
        "leaving Bear Form must remove its Rage modifier"
    );
    assert!(
        stats(&run) < baseline_stamina * 1.01,
        "leaving Bear Form must remove its stamina bonus"
    );
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_incarnation_resets_real_mangle_and_thrash_cooldowns() {
    for spell in [33917, 77758] {
        let mut run = prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(target), false)
            .unwrap();
        for _ in 0..25 {
            run.app.update();
        }
        assert!(
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                false
            )
            .is_err(),
            "{spell} must be on cooldown before Incarnation"
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
            .unwrap();
        for _ in 0..25 {
            run.app.update();
        }
        assert!(
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                false
            )
            .is_ok(),
            "Incarnation must reset {spell}"
        );
    }
}

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_incarnation_shifts_cat_to_bear_before_thrash() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .has_spell(768)
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
        .unwrap();
    let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
    assert!(auras.has_spell(5487));
    assert!(!auras.has_spell(768));
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
}
