//! Galactic Guardian earns a background Moonfire and a charged foreground buff.
use super::*;
use crate::auras::{self, ActiveAuras};
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires Guardian mastery and native world data"]
fn guardian_mastery_follows_non_percent_heals() {
    use crate::spell_cast::effects::{self, EffectCtx};
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::Health;

    let mut run = prepare_registered_player(104, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    assert!(auras::has_aura(run.app.world(), actor, 155783));
    run.app.world_mut().get_mut::<crate::stats::CharacterStats>(actor).unwrap().crit_pct = 0.0;
    let health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
    health.into_inner().current = 1.0;
    run.app.world_mut().resource_mut::<ObservedCombatEvents>().0.clear();
    let heal = |run: &mut Run, spell_id, amount| {
        let spell = crate::spell_cast::load_triggered_spell(run.app.world(), spell_id, 1).unwrap();
        let ctx = EffectCtx { caster: actor, spell: &spell, depth: 1, dest: None, chain_multiplier: 1.0 };
        effects::apply_fixed_heal(run.app.world_mut(), &ctx, actor, amount);
    };
    heal(&mut run, 204069, 10_000.0);
    heal(&mut run, 204069, 20_000.0);
    heal(&mut run, 474683, 10_000.0);
    heal(&mut run, 143924, 10_000.0);
    auras::remove_auras(run.app.world_mut(), actor, auras::RemoveReason::Expired, |a| a.spell_id == 155783);
    heal(&mut run, 204069, 10_000.0);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let mastery: Vec<_> = events.iter().filter(|event| event.spell_id == Some(227034) && event.kind == CombatLogKind::Heal).collect();
    assert_eq!(mastery.len(), 2, "only eligible heals with mastery trigger once");
    assert!(mastery.iter().all(|event| event.source == Some(actor.to_bits()) && event.target == Some(actor.to_bits())));
    assert!(mastery[0].amount > 0);
    assert!((mastery[1].amount - 2 * mastery[0].amount).abs() <= 1);
}

#[test]
#[ignore = "requires Guardian Galactic Guardian profile and native world data"]
fn guardian_galactic_mangle_does_not_proc_from_its_self_effect() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let passive = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 203964 && a.caster == actor)
        .unwrap()
        .instance_id;
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        passive,
        0,
        100.0
    ));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(target), false)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let direct: Vec<_> = events
        .iter()
        .filter(|e| e.spell_id == Some(164812) && e.kind == CombatLogKind::Damage && !e.periodic)
        .collect();
    assert_eq!(
        direct.len(),
        1,
        "one damaging Mangle must earn exactly one Moonfire: {direct:?}"
    );
    assert_eq!(direct[0].target, Some(target.to_bits()));
}

#[test]
#[ignore = "requires Guardian Galactic Guardian profile and native world data"]
fn twin_moonfire_copies_foreground_and_galactic_casts_to_other_enemies() {
    let mut run = prepare_registered_player(104, 3).unwrap();
    let actor = run.actor;
    let primary = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 8921, Some(primary), false)
        .unwrap();
    for _ in 0..3 {
        run.app.update();
    }
    let dotted = |run: &Run| {
        run.targets
            .iter()
            .copied()
            .filter(|target| {
                run.app
                    .world()
                    .get::<ActiveAuras>(*target)
                    .is_some_and(|auras| {
                        auras
                            .iter()
                            .any(|aura| aura.spell_id == 164812 && aura.caster == actor)
                    })
            })
            .collect::<Vec<_>>()
    };
    let first = dotted(&run);
    assert_eq!(first.len(), 2, "foreground Moonfire must hit two targets");
    assert!(first.contains(&primary));
    let passive = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 203964 && aura.caster == actor)
        .unwrap()
        .instance_id;
    for _ in 0..30 {
        run.app.update();
    }
    assert_eq!(dotted(&run).len(), 2);
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        passive,
        0,
        100.0
    ));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(primary), false)
        .unwrap();
    for _ in 0..3 {
        run.app.update();
    }
    assert_eq!(
        dotted(&run).len(),
        3,
        "Galactic Guardian must copy Moonfire to the undotted enemy"
    );
}

#[test]
#[ignore = "requires Guardian Galactic Guardian profile and native world data"]
fn lunar_beam_does_not_consume_galactic_guardian_charge() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let (actor, target) = (run.actor, run.targets[0]);
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let passive = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 203964)
        .unwrap()
        .instance_id;
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        passive,
        0,
        0.0
    ));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 213708, Some(actor), 1);
    let before = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 414613, Some(target), 1);
    for _ in 0..3 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[before..]
            .iter()
            .any(|event| event.spell_id == Some(414613) && event.kind == CombatLogKind::Damage)
    );
    assert!(
        auras::has_aura(run.app.world(), actor, 213708),
        "Lunar Beam must not consume the charge"
    );
    let rage = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 8921, Some(target), false)
        .unwrap();
    assert_eq!(
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
            .unwrap()
            .0
            - rage,
        80.0
    );
    assert!(
        !auras::has_aura(run.app.world(), actor, 213708),
        "foreground Moonfire consumes the charge"
    );
}

#[test]
#[ignore = "requires Guardian Galactic Guardian profile and native world data"]
fn guardian_galactic_proc_casts_and_empowers_moonfire() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let passive = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 203964 && a.caster == actor)
        .unwrap()
        .instance_id;
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        passive,
        0,
        100.0
    ));
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.spell_id == Some(164812)
                    && e.kind == CombatLogKind::Damage
                    && !e.periodic
                    && e.target == Some(target.to_bits())
            })
            .count()
    };
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    assert!(auras::has_aura(run.app.world(), actor, 213708));
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        passive,
        0,
        0.0
    ));
    assert!(
        count(&run) >= 1,
        "actual Thrash proc must apply background Moonfire damage"
    );
    for _ in 0..30 {
        run.app.update();
    }
    let before = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 8921, Some(target), false)
        .unwrap();
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    for _ in 0..3 {
        run.app.update();
    }
    assert!(
        !auras::has_aura(run.app.world(), actor, 213708),
        "foreground Moonfire consumes the buff"
    );
    assert!(
        (after - before - 80.0).abs() < 0.1,
        "native Galactic Guardian gives 8 rage, got {before} -> {after}"
    );
}
