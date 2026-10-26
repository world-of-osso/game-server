//! Native Affliction literal fixture qualification using the pinned MID2 profile.
use super::*;

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_agony_landed_ticks_energize_native_whole_shards() {
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert_eq!(
        crate::spell_cast::powers::power_value(run.app.world(), caster, PowerType::SoulShards)
            .unwrap()
            .0,
        0.0
    );
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: 980,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..360 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage_ticks = events
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(980)
                && event.periodic
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        damage_ticks >= 12,
        "actual landed Agony ticks are the prerequisite: {damage_ticks}"
    );
    let shards =
        crate::spell_cast::powers::power_value(run.app.world(), caster, PowerType::SoulShards)
            .unwrap()
            .0;
    // SimC76f8118 actions1479–1486: each threshold proc pays one displayed shard;
    // native17941 effect0 supplies10raw units, not one Destruction fragment.
    assert!(
        shards >= 10.0 && shards <= 50.0 && shards % 10.0 == 0.0,
        "landed Agony must fund whole native shards: ticks={damage_ticks}, raw={shards}"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_literal_three_target_profile_keeps_foreground_history() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 3).unwrap();
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap()
            .last_successful_cast_at(27243)
            .is_some(),
        "literal three-target precombat must execute actual Seed"
    );
    let combat = report::combat(run.app.world(), &run.targets, 20.0);
    assert!(combat["total_damage"].as_u64().unwrap() > 0);
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_literal_fixture_executes_profile_instead_of_assisted_kit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "265 must capture literal control before native setup"
    );
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 20.0);
    let actions = combat["actions"].as_array().unwrap();
    assert!(
        actions
            .iter()
            .any(|row| row["spell_id"] == 980 && row["damage"].as_u64().unwrap() > 0),
        "literal Agony must damage the actual recipient"
    );
    assert!(
        !actions.iter().any(|row| row["spell_id"] == 702),
        "Curse of Weakness is absent from the pinned APL"
    );
}
