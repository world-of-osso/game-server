//! Incarnation must enter Bear Form from the real Cat Form opener.
use super::*;
use crate::auras::ActiveAuras;

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
