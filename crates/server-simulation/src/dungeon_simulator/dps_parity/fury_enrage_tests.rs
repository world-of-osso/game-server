//! Cruelty and Wrath and Fury are conditional on the live native Enrage aura.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::protocol::CombatLogKind;

const ENRAGE: u32 = 184362;
const CRUELTY: u32 = 392931;
const WRATH_AND_FURY: u32 = 392936;
const RAGING_BLOW_MH: u32 = 96103;

#[derive(Clone, Copy, Debug)]
enum EnragePhase {
    Before,
    During,
    After,
}

fn enter_enrage_phase(run: &mut Run, phase: EnragePhase) {
    let actor = run.actor;
    assert!(!crate::auras::has_aura(run.app.world(), actor, ENRAGE));
    if matches!(phase, EnragePhase::Before) {
        return;
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, ENRAGE, Some(actor), 1);
    assert!(crate::auras::has_aura(run.app.world(), actor, ENRAGE));
    if matches!(phase, EnragePhase::After) {
        for _ in 0..400 {
            run.app.update();
            if !crate::auras::has_aura(run.app.world(), actor, ENRAGE) {
                return;
            }
        }
        panic!("native Enrage must expire within the bounded20s fixture");
    }
}

fn neutralize_enrage_modifiers(run: &mut Run) {
    let actor = run.actor;
    for (spell, index) in [(CRUELTY, 0), (WRATH_AND_FURY, 1)] {
        let instance = run
            .app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .expect("selected native passive")
            .instance_id;
        crate::auras::set_effect_amount(run.app.world_mut(), actor, instance, index, 0.0);
    }
}

fn enrage_hit(phase: EnragePhase, keep_modifiers: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(72, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let cruelty =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, CRUELTY, 0);
    let wrath =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, WRATH_AND_FURY, 1);
    assert!(cruelty > 0.0 && wrath > 0.0);
    enter_enrage_phase(&mut run, phase);
    if !keep_modifiers {
        neutralize_enrage_modifiers(&mut run);
    }
    run.app
        .world_mut()
        .get_mut::<CharacterStats>(actor)
        .unwrap()
        .crit_pct = -1000.0;
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, RAGING_BLOW_MH, Some(target), 1);
    for _ in 0..2 {
        run.app.update();
    }
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(RAGING_BLOW_MH)
                && event.amount > 0
        })
        .collect();
    assert_eq!(hits.len(), 1, "one actual native Raging Blow MH probe");
    assert!(!hits[0].crit);
    (
        hits[0].amount as f64,
        f64::from((1.0 + cruelty / 100.0) * (1.0 + wrath / 100.0)),
    )
}

#[test]
#[ignore = "private current native DB and exact spec72 player inputs"]
fn fury_enrage_gates_cruelty_and_wrath_before_during_and_after_expiry() {
    // SimC76f8118 sc_warrior.cpp1106/4944–4947/5110–5113 and7834–7835.
    for phase in [EnragePhase::Before, EnragePhase::During, EnragePhase::After] {
        let (base, native_factor) = enrage_hit(phase, false);
        let (modified, _) = enrage_hit(phase, true);
        let expected = if matches!(phase, EnragePhase::During) {
            native_factor
        } else {
            1.0
        };
        let actual = modified / base;
        assert!(
            (actual - expected).abs() < 0.002,
            "both selected native passives require live Enrage: phase={phase:?}, actual={actual}, expected={expected}"
        );
    }
}

#[test]
#[ignore = "native player Storm of Blood applies Rend to actual Whirlwind recipients"]
fn fury_storm_of_blood_whirlwind_applies_rend_to_three_targets() {
    for selected in [true, false] {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = prepare_measurement(72, 3).unwrap();
        assert!(
            run.app
                .world()
                .get::<shared::components::Player>(run.actor)
                .is_some()
        );
        engage(&mut run);
        let known = run
            .app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap();
        assert!(
            known.contains(1299025),
            "the measured Fury profile selects Storm of Blood"
        );
        if !selected {
            let spells = known.iter().filter(|id| *id != 1299025).collect();
            run.app
                .world_mut()
                .entity_mut(run.actor)
                .insert(crate::class_progression::KnownSpells::fixture(spells));
        }
        run.app
            .world_mut()
            .resource_mut::<ObservedCombatEvents>()
            .0
            .clear();
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            190411,
            Some(run.targets[0]),
            false,
        )
        .unwrap();
        for _ in 0..100 {
            run.app.update();
            let impacts = run
                .app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .filter(|e| {
                    e.source == Some(run.actor.to_bits())
                        && e.spell_id == Some(199667)
                        && e.kind == CombatLogKind::Damage
                        && e.amount > 0
                })
                .filter_map(|e| e.target)
                .collect::<std::collections::BTreeSet<_>>();
            if impacts.len() == 3 {
                break;
            }
        }
        let impacts = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.source == Some(run.actor.to_bits())
                    && e.spell_id == Some(199667)
                    && e.kind == CombatLogKind::Damage
                    && e.amount > 0
            })
            .filter_map(|e| e.target)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(impacts.len(), 3, "actual first main-hand hits all targets");
        for target in &run.targets {
            assert_eq!(
                crate::auras::has_aura(run.app.world(), *target, 388539),
                selected,
                "Rend on actual target with selected={selected}"
            );
        }
    }
}

#[test]
#[ignore = "requires exact Fury72 native player profile"]
fn fury_enrage_stat_effects_require_their_selected_talents() {
    use crate::class_progression::KnownSpells;
    for (frenzied, powerful) in [(false, false), (true, false), (false, true), (true, true)] {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = prepare_measurement(72, 1).unwrap();
        let actor = run.actor;
        let mut spells: Vec<_> = run
            .app
            .world()
            .get::<KnownSpells>(actor)
            .unwrap()
            .iter()
            .filter(|id| !matches!(id, 383848 | 440277))
            .collect();
        if frenzied {
            spells.push(383848);
        }
        if powerful {
            spells.push(440277);
        }
        run.app
            .world_mut()
            .entity_mut(actor)
            .insert(KnownSpells::fixture(spells));
        let before = run
            .app
            .world()
            .get::<CharacterStats>(actor)
            .unwrap()
            .clone();
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, ENRAGE, Some(actor), 1);
        for _ in 0..2 {
            run.app.update();
        }
        let after = run.app.world().get::<CharacterStats>(actor).unwrap();
        assert!(
            (after.mastery_points - before.mastery_points - if powerful { 15.0 } else { 0.0 })
                .abs()
                < 0.01
        );
        let aura = run
            .app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == ENRAGE)
            .unwrap();
        assert_eq!(
            aura.effects
                .iter()
                .any(|effect| effect.index == 1 && effect.amount == 10.0),
            frenzied
        );
        assert_eq!(
            aura.effects
                .iter()
                .any(|effect| effect.index == 5 && effect.amount == 3.0),
            powerful
        );
        let expected_haste = (1.0 + before.haste_pct / 100.0) * if frenzied { 1.15 } else { 1.0 };
        assert!((1.0 + after.haste_pct / 100.0 - expected_haste).abs() < 0.001);
    }
}

#[test]
#[ignore = "requires exact Fury72 native player profile"]
fn enrage_talent_gate_preserves_non_fury_casters() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(72, 1).unwrap();
    let actor = run.actor;
    run.app
        .world_mut()
        .entity_mut(actor)
        .remove::<shared::components::ActiveSpec>();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, ENRAGE, Some(actor), 1);
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == ENRAGE)
        .unwrap();
    assert!(
        aura.effects
            .iter()
            .any(|effect| effect.index == 4 && effect.amount == 15.0)
    );
    assert!(
        aura.effects
            .iter()
            .any(|effect| effect.index == 5 && effect.amount == 3.0)
    );
}

#[test]
#[ignore = "requires exact Fury72 native player profile"]
fn fury_surge_follows_raging_and_crushing_resets_only_when_selected() {
    for action in [85288, 335097] {
        for selected in [false, true] {
            let _seed = crate::random::seed_scope(70_448);
            let mut run = prepare_measurement(72, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            assert!(crate::auras::has_aura(run.app.world(), actor, 383854));
            assert!(crate::auras::has_aura(run.app.world(), actor, 1265359));
            if !selected {
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    actor,
                    crate::auras::RemoveReason::Consumed,
                    |aura| aura.spell_id == 1265359,
                );
            }
            for _ in 0..40 {
                crate::spell_cast::cast_triggered(
                    run.app.world_mut(),
                    actor,
                    action,
                    Some(target),
                    1,
                );
                if crate::auras::has_aura(run.app.world(), actor, 1265560) {
                    break;
                }
            }
            assert_eq!(
                crate::auras::has_aura(run.app.world(), actor, 1265560),
                selected,
                "action={action} selected={selected}"
            );
            if selected {
                let aura = run
                    .app
                    .world()
                    .get::<ActiveAuras>(actor)
                    .unwrap()
                    .iter()
                    .find(|aura| aura.spell_id == 1265560)
                    .unwrap();
                assert!(
                    aura.effects
                        .iter()
                        .any(|effect| effect.index == 0 && effect.amount == 30.0)
                );
                assert!(
                    aura.effects
                        .iter()
                        .any(|effect| effect.index == 1 && effect.amount == 30.0)
                );
            }
        }
    }
}

#[test]
#[ignore = "requires exact Fury72 native player profile"]
fn fury_rampaging_berserker_changes_paid_cost_and_hit_only_during_recklessness() {
    use super::literal_rotation::execution::{Execution, execute};
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    fn probe(selected: bool, reckless: bool) -> (f32, i32) {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = prepare_measurement(72, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), actor, 1269309));
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 392792 || (!selected && aura.spell_id == 1269309),
        );
        if reckless {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1719, Some(actor), 1);
        }
        run.app
            .world_mut()
            .get_mut::<CharacterStats>(actor)
            .unwrap()
            .crit_pct = -1000.0;
        run.app
            .world_mut()
            .get_mut::<PowerPools>(actor)
            .unwrap()
            .get_mut(PowerType::Rage)
            .unwrap()
            .current = 1000.0;
        run.app
            .world_mut()
            .resource_mut::<ObservedCombatEvents>()
            .0
            .clear();
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(184367, target, Some(target)),
        )
        .unwrap();
        let remaining = run
            .app
            .world()
            .get::<PowerPools>(actor)
            .unwrap()
            .get(PowerType::Rage)
            .unwrap()
            .current;
        for _ in 0..40 {
            run.app.update();
        }
        let hit = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .find(|e| {
                e.source == Some(actor.to_bits())
                    && e.spell_id == Some(201363)
                    && e.kind == CombatLogKind::Damage
                    && e.amount > 0
            })
            .expect("native final Rampage hit")
            .amount;
        (1000.0 - remaining, hit)
    }
    let ordinary = probe(false, false);
    let talent_only = probe(true, false);
    let reckless_only = probe(false, true);
    let both = probe(true, true);
    assert_eq!(ordinary.0, 800.0);
    assert_eq!(talent_only.0, 800.0);
    assert_eq!(reckless_only.0, 800.0);
    assert_eq!(both.0, 500.0);
    assert!(
        (both.1 as f32 / reckless_only.1 as f32 - 1.10).abs() < 0.01,
        "native hit selected={} unselected={}",
        both.1,
        reckless_only.1
    );
}
