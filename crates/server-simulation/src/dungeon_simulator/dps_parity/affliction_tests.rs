//! Native Affliction literal fixture qualification using the pinned MID2 profile.
use super::*;

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_wither_landed_ticks_earn_native_nightfall() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, 108558));
    let mut earned = false;
    for _ in 0..10 {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster,
                target,
                spell_id: 445474,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        for _ in 0..100 {
            run.app.update();
        }
        earned = crate::auras::has_aura(run.app.world(), caster, 264571);
        if earned {
            break;
        }
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(445474)
                && event.periodic
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0)
    );
    // SimC76f8118 init1190–1225: actual Wither replaces Corruption in this threshold stream.
    assert!(
        earned,
        "actual owned Wither ticks must earn selected native Nightfall"
    );
    let before = super::affliction_nightfall_tests::uses(&run);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 686, Some(target), false)
        .unwrap();
    assert_eq!(
        super::affliction_nightfall_tests::uses(&run),
        before - 1,
        "one admitted instant Bolt spends one earned use, not every stored use"
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none(),
        "earned native Nightfall makes the ordinary Shadow Bolt instant"
    );
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(686)
                && event.amount > 0
                && event.kind == shared::protocol::CombatLogKind::Damage
        }),
        "the admitted instant Shadow Bolt must actually land"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_shadow_bolt_landed_hits_earn_native_shard_instability() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, 1260264));
    for _ in 0..40 {
        crate::spell_cast::creature_cast(run.app.world_mut(), caster, 686, Some(target), false)
            .unwrap();
        for _ in 0..70 {
            run.app.update();
        }
        if crate::auras::has_aura(run.app.world(), caster, 1260269) {
            break;
        }
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(686)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0)
    );
    // SimC76f8118 sc_warlock_actions.cpp887–890/init1242–1245: selected talent,
    // landed Shadow Bolt, native effect1 PRD chance and actual1260269 buff.
    assert!(
        crate::auras::has_aura(run.app.world(), caster, 1260269),
        "landed native Shadow Bolts must earn the selected Shard Instability buff"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 1259790, Some(target), false)
        .expect("earned Instability must admit free native Unstable Affliction at zero shards");
    for _ in 0..10 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), caster, 1260269),
        "actual instant UA must consume its admitted Instability use"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_selected_malefic_grasp_projects_to_real_native_channel() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let caster = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(caster)
            .unwrap()
            .contains(1261149)
    );
    // SimC76f8118 init181–183/actions2100: selected1261149 enables channel1261153.
    let spell = crate::spell_cast::knowledge::resolve_actor_action(
        run.app.world(),
        caster,
        1261149,
    )
    .expect(
        "selected Malefic Grasp must resolve its native channel instead of its passive talent root",
    );
    assert_eq!(spell, 1261153);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 205180, Some(caster), true)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, spell, Some(target), false)
        .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .unwrap()
            .cast_type,
        shared::casting::CastType::Channel
    );
    for _ in 0..60 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(1261153)
                && event.periodic
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0),
        "the selected talent must enable actual periodic channel damage"
    );
}

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
