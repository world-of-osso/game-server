//! SimC76f8118 sc_priest.cpp4285–4341: live owned DoTs supply Shadow Weaving.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn mastery_amount(run: &Run) -> f64 {
    let auras = run.app.world().get::<ActiveAuras>(run.actor).unwrap();
    let mastery = auras
        .iter()
        .find(|aura| aura.spell_id == 343690 && aura.caster == run.actor)
        .unwrap();
    let effect = mastery
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    f64::from(effect.amount) / 100.0
}

fn apply_three_owned_dots(run: &mut Run) {
    for (spell_id, effect_mask) in [(589, 1 << 1), (34914, 1 << 1), (335467, 1 << 2)] {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: run.actor,
                target: run.targets[0],
                spell_id,
                effect_mask,
                source: AuraSource::Cast,
            },
        );
        assert!(
            crate::auras::has_aura(run.app.world(), run.targets[0], spell_id),
            "native owned DoT{spell_id} must be applied before the cast"
        );
    }
}

fn mind_blast_damage(dotted: bool) -> (f64, bool, f64) {
    let mut run = prepare_literal(258, 1).unwrap();
    let mastery = mastery_amount(&run);
    assert!(mastery > 0.0);
    if dotted {
        apply_three_owned_dots(&mut run);
    }
    // Seed after fixture setup; no fabricated crit, stats, power or damage inputs.
    let _seed = crate::random::seed_scope(70448);
    engage_fixture_opponents(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(8092, run.targets[0], None),
    )
    .unwrap();
    let (damage, critical) = wait_for_mind_blast(&mut run);
    (damage, critical, mastery)
}

fn wait_for_mind_blast(run: &mut Run) -> (f64, bool) {
    for _ in 0..100 {
        run.app.update();
        let hit = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(8092)
                    && event.kind == CombatLogKind::Damage
            });
        if let Some(hit) = hit {
            return (f64::from(hit.amount), hit.crit);
        }
    }
    panic!("ordinary native Mind Blast did not hit");
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_mastery_scales_real_mind_blast_by_three_live_owned_dots() {
    let (bare, bare_crit, mastery) = mind_blast_damage(false);
    let (dotted, dotted_crit, dotted_mastery) = mind_blast_damage(true);
    assert_eq!(bare_crit, dotted_crit, "same native seeded critical result");
    assert_eq!(mastery, dotted_mastery);
    assert!(bare > 0.0);
    let expected = bare * (1.0 + 3.0 * mastery);
    assert!(
        (dotted - expected).abs() <= 3.0,
        "native mastery={mastery}: bare={bare}, dotted={dotted}, expected={expected}"
    );
}
