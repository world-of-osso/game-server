//! Aura-level native channel refresh; foreground recast admission is not under test.
use super::*;
use crate::auras::{AuraApplication, AuraInstance, AuraSource, RemoveReason, SpellModOp};
use crate::spell_info::{ChargedSpellModifier, SpellInfo};

const GRASP: u32 = 1261153;
const NIGHTFALL: u32 = 264571;

fn owned_grasp(run: &Run) -> Option<AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])?
        .iter()
        .find(|aura| aura.spell_id == GRASP && aura.caster == run.actor)
        .cloned()
}

fn native_grasp_snapshot(run: &mut Run) -> (SpellInfo, f32) {
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: NIGHTFALL,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let world = run.app.world();
    let aura = world
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == NIGHTFALL && aura.caster == run.actor)
        .unwrap();
    let factor = 1.0
        + aura
            .effects
            .iter()
            .find(|effect| effect.misc_values[0] == SpellModOp::Period as i32)
            .unwrap()
            .amount
            / 100.0;
    // Replay actual native modifier state through the source-owned spell API.
    // No points, haste, clocks, charges or native rows are rewritten.
    let modifiers = aura
        .effects
        .iter()
        .filter(|effect| {
            matches!(
                effect.aura_type,
                crate::auras::handlers::ADD_PCT_MODIFIER
                    | crate::auras::handlers::ADD_FLAT_MODIFIER
            )
        })
        .map(|effect| ChargedSpellModifier {
            aura_spell_id: aura.spell_id,
            op: effect.misc_values[0],
            amount: effect.amount,
            percent: effect.aura_type == crate::auras::handlers::ADD_PCT_MODIFIER,
        })
        .collect();
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(GRASP)
        .unwrap()
        .clone();
    spell.cast_charge_mods = Some(modifiers);
    (spell, factor)
}

#[test]
#[ignore = "native265 channel Pandemic candidate; requires private native profile/DB"]
fn native_pandemic_grasp_carry_retains_charged_period_snapshot() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 205180, Some(run.actor), 0);
    let (spell, factor) = native_grasp_snapshot(&mut run);
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &spell,
        Some(run.targets[0]),
        0
    ));
    // The public triggered route does not spend foreground class uses.
    // Remove the applied native use after launch to exercise retained modifiers.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        RemoveReason::Consumed,
        |aura| aura.spell_id == NIGHTFALL,
    );
    let initial = owned_grasp(&run).unwrap();
    let old_expiry = initial.expires_at.unwrap();
    let refresh_window = f64::from(initial.duration_ms) / 1000.0 * 0.20;
    for _ in 0..200 {
        let remaining = old_expiry - crate::auras::aura_now(run.app.world());
        if remaining > 0.0 && remaining <= refresh_window {
            break;
        }
        run.app.update();
    }
    let now = crate::auras::aura_now(run.app.world());
    let remaining = old_expiry - now;
    assert!(remaining > 0.0, "refresh must precede the native expiry");
    let haste = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    let base_period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    let base_duration = spell.misc.as_ref().unwrap().duration.unwrap().duration_ms as u32;
    assert_eq!(base_duration / base_period, 4);
    let period = (base_period as f32 * factor / (1.0 + haste / 100.0)).round() as u32;
    let fresh = f64::from(4 * period) / 1000.0;
    let expected_expiry = now + remaining.max(fresh + remaining.min(0.30 * fresh));
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &spell,
        Some(run.targets[0]),
        0
    ));
    let refreshed = owned_grasp(&run).unwrap();
    // SimC76f8118 action.cpp684–685/4612/4630–4635: native attr13 has no channel exclusion.
    assert!(
        (refreshed.expires_at.unwrap() - expected_expiry).abs() < 0.001,
        "native Grasp refresh must retain Pandemic carry: actual{:?}, expected{expected_expiry}",
        refreshed.expires_at
    );
    for _ in 0..200 {
        if crate::auras::aura_now(run.app.world()) >= expected_expiry {
            break;
        }
        let aura = owned_grasp(&run).expect("carried channel aura cannot expire early");
        assert_eq!(
            aura.effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .period_ms,
            period,
            "refresh/rebase retains the charged modifier and its application haste"
        );
        run.app.update();
    }
    for _ in 0..3 {
        run.app.update();
    }
    assert!(
        owned_grasp(&run).is_none(),
        "carried native channel remains finite"
    );
}
