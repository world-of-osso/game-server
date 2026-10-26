//! Native crit-only input changes actual guaranteed-critical spell damage.
use super::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::protocol::CombatLogKind;

const DARK_SOUL: u32 = 113858;
const MASTERY: u32 = 77220;

fn actual_hit(spell_id: u32, boost: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    // Remove the separately qualified random mastery for a same-seed,
    // crit-only comparison. Do not change a native coefficient or stat value.
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        RemoveReason::Consumed,
        |aura| aura.spell_id == MASTERY,
    );
    let before = *run.app.world().get::<CharacterStats>(caster).unwrap();
    if boost {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster,
                target: caster,
                spell_id: DARK_SOUL,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    // Aura application marks modifiers changed; the real stat system folds
    // them on the next frame. Advance both samples equally before admission.
    run.app.update();
    if boost {
        let after = run.app.world().get::<CharacterStats>(caster).unwrap();
        assert_eq!(
            after.spell_power, before.spell_power,
            "native control changes no spell power"
        );
        assert_eq!(
            after.versatility_pct, before.versatility_pct,
            "native control changes no versatility"
        );
        assert!(
            (after.crit_pct - before.crit_pct - 30.0).abs() < 0.01,
            "native113858 must supply its actual single-effect30% crit input: before={}, after={}",
            before.crit_pct,
            after.crit_pct
        );
    }
    if spell_id == 17877 {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max * 0.19;
    }
    let crit = f64::from(
        run.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .crit_pct,
    );
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        caster,
        &spell,
        Some(target),
        1
    ));
    for _ in 0..60 {
        run.app.update();
        let hit = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(spell_id)
                    && event.kind == CombatLogKind::Damage
            });
        if let Some(hit) = hit {
            assert!(
                hit.crit && hit.amount > 0,
                "both native samples must actually crit"
            );
            return (f64::from(hit.amount), crit);
        }
    }
    panic!("native spell{spell_id} must land");
}

fn assert_native_crit_scaling(spell: u32) {
    let (base_damage, base_crit) = actual_hit(spell, false);
    let (boost_damage, boost_crit) = actual_hit(spell, true);
    let expected_ratio = (1.0 + boost_crit / 100.0) / (1.0 + base_crit / 100.0);
    let expected = base_damage * expected_ratio;
    assert!(
        (boost_damage - expected).abs() <= 3.0,
        "spell{spell}: native crit{base_crit}→{boost_crit} must scale an actual guaranteed crit: damage{base_damage}→{boost_damage}, source ratio{expected_ratio}, expected{expected}"
    );
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_coc_native_crit_chance_scales_actual_conflagrate_and_shadowburn() {
    // SimC76f8118 actions4511–4517/4838–4844: selected Conflagration of
    // Chaos uses live caster crit for damage, beyond guaranteeing the crit.
    for spell in [17962, 17877] {
        assert_native_crit_scaling(spell);
    }
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_coc_existing_chaos_bolt_crit_scaling_remains_a_native_control() {
    // actions4413–4416; existing native batch5 scaling is not a missing producer.
    assert_native_crit_scaling(116858);
}
