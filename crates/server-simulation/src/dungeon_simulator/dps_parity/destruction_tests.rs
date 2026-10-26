//! Native Destruction fixture control and actual damage, using the pinned profile.
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private267 profile and native world DB"]
fn destruction_periodic_damage_funds_real_soul_shard_pool() {
    // SimC76f8118 sc_warlock_actions.cpp1111–1117/4051–4059: one
    // fragment per landed Wither/Immolate tick, plus a possible critical fragment.
    for spell_id in [445474, 157736] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(267, 1).unwrap();
        let (caster, target) = (run.actor, run.targets[0]);
        let before = crate::spell_cast::powers::power_value(run.app.world(), caster, PowerType::SoulShards).unwrap().0;
        crate::auras::apply_aura(run.app.world_mut(), &crate::auras::AuraApplication {
            caster, target, spell_id, effect_mask: 1, source: crate::auras::AuraSource::Cast,
        });
        let mut ticked = false;
        for _ in 0..100 {
            run.app.update();
            ticked = run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter().any(|event| {
                event.source == Some(caster.to_bits()) && event.target == Some(target.to_bits())
                    && event.spell_id == Some(spell_id) && event.periodic
                    && event.kind == CombatLogKind::Damage && event.amount > 0
            });
            if ticked { break; }
        }
        assert!(ticked, "actual native periodic{spell_id} damage is the prerequisite");
        let after = crate::spell_cast::powers::power_value(run.app.world(), caster, PowerType::SoulShards).unwrap().0;
        assert!(after >= before + 1.0 && after <= before + 2.0,
            "first landed native{spell_id} tick must fund1–2 raw shard units: before{before}, after{after}");
    }
}

#[test]
#[ignore = "requires private267 profile and native world DB"]
fn destruction_literal_fixture_executes_profile_instead_of_native_kit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(267, 1).unwrap();
    assert!(
        run.app.world().get::<literal_rotation::LiteralParityFixture>(run.actor).is_some(),
        "267 must capture literal control before native setup"
    );
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 20.0);
    let actions = combat["actions"].as_array().unwrap();
    assert!(actions.iter().any(|row| row["spell_id"] == 29722 && row["damage"].as_u64().unwrap() > 0),
        "literal Incinerate must damage the actual recipient");
    assert!(!actions.iter().any(|row| row["spell_id"] == 702),
        "Curse of Weakness is absent from the pinned APL");
}
