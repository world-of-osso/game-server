//! Real Conflagrate damage isolates native Chaotic Energies from coefficient data.
use super::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const MASTERY: u32 = 77220;
const CONFLAGRATE: u32 = 17962;
const CRITICAL_FOCUS: u32 = 75060;

fn cast_native_conflagrate(keep_mastery: bool) -> (f64, f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    let (actor, target) = (run.actor, run.targets[0]);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: CRITICAL_FOCUS,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    run.app.update();
    assert!(crate::auras::has_aura(run.app.world(), actor, MASTERY));
    if !keep_mastery {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            RemoveReason::Dispelled,
            |aura| aura.spell_id == MASTERY,
        );
    }
    assert_eq!(
        crate::auras::has_aura(run.app.world(), actor, MASTERY),
        keep_mastery
    );
    let mastery = f64::from(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_pct,
    ) / 100.0;
    if keep_mastery {
        assert!(mastery > 0.0);
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(CONFLAGRATE, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let hit = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(CONFLAGRATE)
                && event.kind == CombatLogKind::Damage
        })
        .expect("native Conflagrate must actually damage the selected recipient");
    assert!(
        hit.amount > 0 && hit.crit,
        "both samples must retain the actual native critical outcome"
    );
    assert_eq!(
        crate::auras::has_aura(run.app.world(), actor, MASTERY),
        keep_mastery,
        "the native mastery aura is the controlled difference, not edited stats or spell rows"
    );
    let variance = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(CONFLAGRATE)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .variance;
    (f64::from(hit.amount), mastery, f64::from(variance))
}

#[test]
#[ignore = "requires private267 native profile and world DB"]
fn destruction_mastery_modifies_actual_native_conflagrate_damage() {
    // SimC76f8118 sc_warlock_actions.cpp480–489/4443: Conflagrate
    // receives1+U(0.5,1)*mastery, independently of the native SP coefficient.
    let (control, _, variance) = cast_native_conflagrate(false);
    let (empowered, mastery, _) = cast_native_conflagrate(true);
    let spread = (1.0 + variance / 2.0) / (1.0 - variance / 2.0);
    let minimum = control * (1.0 + 0.5 * mastery) / spread;
    let maximum = control * (1.0 + mastery) * spread;
    assert!(
        empowered >= minimum - 2.0 && empowered <= maximum + 2.0,
        "actual native mastery{mastery}: control{control}, empowered{empowered}, source range{minimum}..{maximum}, native variance{variance}"
    );
}

#[test]
#[ignore = "requires private267 native profile and world DB"]
fn destruction_mastery_prediction_uses_mean_without_advancing_native_rng() {
    use server::simc_apl::Facts;
    let mut run = prepare_literal(267, 1).unwrap();
    let bindings = crate::simc_apl_native::Bindings {
        actions: [
            ("conflagrate".into(), CONFLAGRATE),
            ("chaos_bolt".into(), 116858),
        ]
        .into(),
        ..Default::default()
    };
    let forecast = |run: &Run, alias: &str| {
        crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            crate::simc_apl_native::Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: crate::simc_apl_native::FightStyle::Patchwerk,
            },
            1,
        )
        .with_targets(Some(run.targets[0]), &run.targets)
        .value(&format!("action.{alias}.damage"))
        .unwrap()
    };
    let next_without_forecast = {
        let _seed = crate::random::seed_scope(70448);
        crate::random::random::<f32>()
    };
    let incarnate_lower = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(387275)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    let cases = [
        ("conflagrate", 0.5),
        ("chaos_bolt", f64::from(incarnate_lower) / 100.0),
    ];
    let with_mastery = cases.map(|(alias, lower)| {
        let _seed = crate::random::seed_scope(70448);
        let first = forecast(&run, alias);
        assert_eq!(
            forecast(&run, alias),
            first,
            "the expected snapshot must be deterministic"
        );
        assert_eq!(
            crate::random::random::<f32>(),
            next_without_forecast,
            "read-only damage facts cannot consume native combat RNG"
        );
        (alias, lower, first)
    });
    let mastery = f64::from(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .mastery_pct,
    ) / 100.0;
    assert!(mastery > 0.0);
    let actor = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        RemoveReason::Dispelled,
        |aura| aura.spell_id == MASTERY,
    );
    for (alias, lower, empowered) in with_mastery {
        let control = forecast(&run, alias);
        let mean = (lower + 1.0) / 2.0;
        let expected = control * (1.0 + mean * mastery);
        assert!(
            (empowered - expected).abs() <= 1.0,
            "native{alias} mastery mean: control{control}, forecast{empowered}, expected{expected}"
        );
    }
}
