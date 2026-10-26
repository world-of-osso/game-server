//! Actual native Lance/Spike factor observations against pinned Mage consumer rules.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn apply(run: &mut Run, spell: u32, target: usize) {
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: if target == 0 {
                run.actor
            } else {
                run.targets[target - 1]
            },
            spell_id: spell,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn freezing(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 1221389 && aura.caster == run.actor)
        })
        .map_or(0, |aura| aura.stacks)
}

fn prepare_case(stacks: u8, buffs: &[u32]) -> Run {
    let mut run = prepare_literal(64, 3).unwrap();
    // A real opener has already advanced past Deep Shatter's native1s update.
    // Do not mistake its setup-time modifier for a steady-state per-hit gap.
    for _ in 0..40 {
        run.app.update();
    }
    // Isolate consumption from independently earned Hand/Infused/tier followups.
    // Retain native damage, crit, stat, stack-limit and projectile definitions.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| {
            matches!(
                aura.spell_id,
                44544 | 1247730 | 455122 | 1262935 | 1261080 | 1296585
            )
        },
    );
    for _ in 0..stacks {
        apply(&mut run, 1221389, 1);
    }
    for &buff in buffs {
        apply(&mut run, buff, 0);
    }
    run
}

fn capture_hit(run: &mut Run, root: u32, focus: u32, label: &str) -> serde_json::Value {
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        focus,
        label,
        1,
    ));
    let _seed = crate::random::seed_scope(64070448);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(root, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    for _ in 0..150 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(focus)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            return run.app.world().resource::<FactorCapture>().export();
        }
    }
    panic!("actual native{root} must land payload{focus}");
}

#[test]
#[ignore = "native70448 Frost per-hit factor investigation"]
fn frost_shatter_factor_native_lance_consumption_and_multiplier_matrix() {
    // SimC76f8118 sc_mage.cpp4445–4473/7129–7169: Fingers grants the
    // full limit without consuming Freezing; Thermal doubles that limit.
    for (label, stacks, buffs) in [
        ("ordinary_six", 6, &[][..]),
        ("fingers_six", 6, &[44544][..]),
        ("fingers_without_freezing", 0, &[44544][..]),
        ("thermal_twelve", 12, &[1247730][..]),
        ("fingers_thermal_three", 3, &[44544, 1247730][..]),
        ("permafrost_six", 6, &[455122][..]),
    ] {
        let _fixture_seed = crate::random::seed_scope(70448);
        let mut run = prepare_case(stacks, buffs);
        let before = freezing(&run);
        let stats = report::stats(run.app.world(), run.actor);
        let trace = capture_hit(&mut run, 30455, 228598, label);
        println!(
            "FROST_SHATTER_FACTOR {}",
            serde_json::json!({
                "case": label, "freezing_before": before, "freezing_after": freezing(&run),
                "stats": stats, "trace": trace,
            })
        );
    }
}

#[test]
#[ignore = "native70448 Frost per-hit factor investigation"]
fn frost_shatter_factor_native_spike_preserves_unselected_glacial_shatter() {
    let _fixture_seed = crate::random::seed_scope(70448);
    let mut run = prepare_case(6, &[]);
    for _ in 0..5 {
        apply(&mut run, 205473, 0);
    }
    apply(&mut run, 1222865, 0);
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1280780));
    let before = freezing(&run);
    let trace = capture_hit(&mut run, 199786, 228600, "spike_without_glacial_shatter");
    println!(
        "FROST_SHATTER_FACTOR {}",
        serde_json::json!({
            "case": "spike_without_glacial_shatter", "freezing_before": before,
            "freezing_after": freezing(&run), "trace": trace,
        })
    );
}
