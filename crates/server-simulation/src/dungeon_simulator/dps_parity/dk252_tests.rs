use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_festering_strike_earns_ghouls_and_next_native_scythe() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    let cast_strike = |run: &mut Run| {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                85948,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(run).unwrap();
    };
    cast_strike(&mut run);
    let ready = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1254252 && aura.caster == run.actor)
        .expect("Festering Strike must earn the native ready-ghoul counter")
        .stacks;
    assert!(
        (2..=3).contains(&ready),
        "native effects2/3 supply two or three ghouls"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 458123));
    cast_strike(&mut run);
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1241077),
        "the real replacement Scythe must open its native tracking window"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(458128)
                && event.kind == CombatLogKind::CastSuccess)
    );
}

#[test]
#[ignore = "requires private native252 Rider templates/profile and player actor"]
fn dk252_army_summons_all_four_real_riders() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    ).unwrap();
    for _ in 0..100 { run.app.update(); }
    let riders: std::collections::HashSet<_> = {
        let world = run.app.world_mut();
        world.query::<(&shared::components::Npc, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world).filter_map(|(npc, summon)|
                (summon.owner == run.actor && [444248, 444251, 444252, 444254].contains(&summon.spell_id))
                    .then_some(npc.template_id)
            ).collect()
    };
    assert_eq!(riders, [221632, 221633, 221634, 221635].into_iter().collect(),
        "selected Apocalypse Now must summon all four actual native Horsemen");
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_real_lesser_ghoul_summon_reaches_and_melees() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        275430,
        Some(run.actor),
        1,
    ));
    let mut ghouls = std::collections::HashSet::new();
    for _ in 0..200 {
        run.app.update();
        let world = run.app.world_mut();
        ghouls.extend(
            world
                .query::<(
                    Entity,
                    &shared::components::Npc,
                    &crate::spell_cast::summon::SummonedCreature,
                )>()
                .iter(world)
                .filter_map(|(entity, npc, summon)| {
                    (npc.template_id == 237409 && summon.owner == run.actor)
                        .then_some(entity.to_bits())
                }),
        );
    }
    assert_eq!(
        ghouls.len(),
        1,
        "the native summon must create a real lesser ghoul"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(
            |event| event.source.is_some_and(|source| ghouls.contains(&source))
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0
        ),
        "the actual lesser ghoul must reach and melee the enemy"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_lesser_ghoul_melee_inherits_live_owner_attack_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        275430,
        Some(run.actor),
        1,
    ));
    let ghoul = (0..100)
        .find_map(|_| {
            run.app.update();
            let world = run.app.world_mut();
            world
                .query::<(Entity, &shared::components::Npc)>()
                .iter(world)
                .find_map(|(entity, npc)| (npc.template_id == 237409).then_some(entity))
        })
        .expect("native lesser ghoul must exist");
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .attack_power += 10_000.0;
    for _ in 0..200 {
        run.app.update();
    }
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(ghoul.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0
        })
        .map(|event| event.amount)
        .collect();
    assert!(
        !hits.is_empty(),
        "the real lesser ghoul must land melee hits"
    );
    assert!(
        hits.iter().any(|hit| *hit > 100),
        "native ghoul swings must include the live owner's melee AP: {hits:?}"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_army_cast_summons_real_melee_ghouls() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    let mut ghouls = std::collections::HashSet::new();
    for _ in 0..300 {
        run.app.update();
        let world = run.app.world_mut();
        ghouls.extend(
            world
                .query::<(
                    Entity,
                    &shared::components::Npc,
                    &crate::spell_cast::summon::SummonedCreature,
                )>()
                .iter(world)
                .filter_map(|(entity, npc, summon)| {
                    (npc.template_id == 237409
                        && summon.owner == run.actor
                        && summon.spell_id == 1294026)
                        .then_some(entity.to_bits())
                }),
        );
    }
    assert_eq!(
        ghouls.len(),
        8,
        "Army must summon eight real native Army ghouls"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(
            |event| event.source.is_some_and(|source| ghouls.contains(&source))
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0
        ),
        "an actual Army ghoul must reach and melee the enemy"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_dread_plague_ticks_raise_forbidden_knowledge_magi() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        1240996,
        Some(run.targets[0]),
        1,
    ));
    for _ in 0..600 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(1240996)
            && event.target == Some(run.targets[0].to_bits())
            && event.kind == CombatLogKind::Damage
            && event.amount > 0),
        "the actual Dread Plague must deal periodic damage"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1277098)
                && event.kind == CombatLogKind::Summon),
        "selected Forbidden Knowledge must raise actual Putrefy ghouls from Dread Plague ticks"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1242294)
                && event.kind == CombatLogKind::Summon),
        "actual Forbidden Knowledge ghouls must raise native Lord Magi"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_putrefy_summons_native_lesser_ghoul() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1277098)
                && event.kind == CombatLogKind::Summon
                && event.target.is_some()),
        "Putrefy must summon its native lesser ghoul"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_putrefy_ghoul_impacts_the_enemy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..300 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(1277016)
            && event.target == Some(run.targets[0].to_bits())
            && event.kind == CombatLogKind::Damage
            && event.amount > 0),
        "the summoned Putrefy ghoul must deliver native single-target impact damage"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_putrefy_impact_raises_native_magus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1256813));
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let world = run.app.world_mut();
    assert!(
        world
            .query::<&shared::components::Npc>()
            .iter(world)
            .any(|npc| npc.template_id == 163366),
        "selected Lord of the Dead must raise a native Magus after Putrefy impact"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_putrefy_magus_casts_native_necrotic_bolt() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..300 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(1297055)
            && event.target == Some(run.targets[0].to_bits())
            && event.kind == CombatLogKind::Damage
            && event.amount > 0),
        "the actual native Magus must damage the enemy with Necrotic Bolt"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_putrid_echoes_spends_both_charges_and_summons_two_ghouls() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 377580));
    let bindings = Bindings {
        actions: [("putrefy".into(), 1247378)].into(),
        ..Default::default()
    };
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let charges = |run: &Run| {
        NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
            .value("cooldown.putrefy.charges")
            .unwrap()
    };
    let initial_charges = charges(&run);
    assert!(
        initial_charges >= 2.0,
        "the fixture starts with at least two real Putrefy charges"
    );
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let count = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1277098)
                && event.kind == CombatLogKind::Summon
        })
        .count();
    assert_eq!(
        count, 2,
        "selected Putrid Echoes must create two native lesser ghouls"
    );
    assert_eq!(
        charges(&run),
        initial_charges - 2.0,
        "the second ghoul spends another real Putrefy charge"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_three_real_magi_raise_the_lord_of_the_dead() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    for _ in 0..2 {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                1247378,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        for _ in 0..50 {
            run.app.update();
        }
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let summons = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Summon
            })
            .count()
    };
    assert_eq!(
        summons(1242294),
        3,
        "actual Putrefy casts must produce three native Magi"
    );
    assert_eq!(
        summons(1292072),
        1,
        "three actual Magi must be consumed to summon one native Lord of the Dead"
    );
    let world = run.app.world_mut();
    assert!(
        !world
            .query::<&shared::components::Npc>()
            .iter(world)
            .any(|npc| npc.template_id == 163366),
        "the three source Magi must actually be dismissed"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_real_lord_casts_necrotic_bolt_using_owner_power() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    for _ in 0..2 {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                1247378,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        for _ in 0..50 {
            run.app.update();
        }
    }
    let lord = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &shared::components::Npc)>()
            .iter(world)
            .find_map(|(actor, npc)| (npc.template_id == 264321).then_some(actor))
            .expect("three actual Magi must create the native Lord")
    };
    let owner = run.actor;
    let target = run.targets[0];
    run.app.insert_resource(FactorCapture::new(
        lord,
        target,
        1297086,
        "lord_necrotic_bolt",
        1,
    ));
    for _ in 0..200 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        let Some(calculation) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1297086)
        else {
            continue;
        };
        let world = run.app.world();
        let stats = world.get::<crate::stats::CharacterStats>(owner).unwrap();
        let expected = stats.total_attack_power(
            crate::stats::WeaponAttackType::MainHand,
            world.get::<crate::stats::StatModifiers>(owner),
        ) * 0.7;
        let inherited = calculation["factors"]["a_power"].as_f64().unwrap();
        assert!(
            (inherited - f64::from(expected)).abs() < 1.0,
            "actual Lord bolt must inherit owner melee AP: actual={inherited}, expected={expected}"
        );
        return;
    }
    panic!("the actual Lord must complete a native Necrotic Bolt");
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_lord_magus_uses_selected_native_lifetime() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    let magus = (0..100)
        .find_map(|_| {
            run.app.update();
            let world = run.app.world_mut();
            world
                .query::<(Entity, &shared::components::Npc)>()
                .iter(world)
                .find_map(|(actor, npc)| (npc.template_id == 163366).then_some(actor))
        })
        .expect("native Putrefy must create a Magus");
    let native_ms =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1256813, 1);
    let started = crate::spell_cast::now(run.app.world());
    let observed_expiry = run
        .app
        .world()
        .get::<crate::spell_cast::summon::SummonedCreature>(magus)
        .unwrap()
        .expires_at;
    let virtual_now = crate::auras::aura_now(run.app.world());
    while crate::spell_cast::now(run.app.world()) < started + f64::from(native_ms) / 1000.0 - 1.0 {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(magus).is_ok(),
        "the real Magus must survive until its selected native lifetime: native_ms={native_ms}, start={started}, virtual_start={virtual_now}, expiry={observed_expiry:?}, current={}, virtual_current={}",
        crate::spell_cast::now(run.app.world()),
        crate::auras::aura_now(run.app.world())
    );
    while crate::spell_cast::now(run.app.world()) < started + f64::from(native_ms) / 1000.0 + 1.0 {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(magus).is_err(),
        "the real Magus must expire after its selected native lifetime"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_magus_bolt_inherits_owner_melee_power() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let magus = (0..100)
        .find_map(|_| {
            run.app.update();
            let world = run.app.world_mut();
            world
                .query::<(Entity, &shared::components::Npc)>()
                .iter(world)
                .find_map(|(actor, npc)| (npc.template_id == 163366).then_some(actor))
        })
        .expect("native Putrefy must create a Magus");
    let owner = run.actor;
    let target = run.targets[0];
    run.app.insert_resource(FactorCapture::new(
        magus,
        target,
        1297055,
        "necrotic_bolt",
        1,
    ));
    for _ in 0..200 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        let Some(calculation) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1297055)
        else {
            continue;
        };
        let world = run.app.world();
        let stats = world.get::<crate::stats::CharacterStats>(owner).unwrap();
        let expected = stats.total_attack_power(
            crate::stats::WeaponAttackType::MainHand,
            world.get::<crate::stats::StatModifiers>(owner),
        ) * 0.4664;
        let inherited = calculation["factors"]["a_power"].as_f64().unwrap();
        assert!(
            (inherited - f64::from(expected)).abs() < 1.0,
            "actual Magus bolt must inherit owner melee AP: actual={inherited}, expected={expected}"
        );
        return;
    }
    panic!("the actual Magus must complete a native Necrotic Bolt");
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_dark_transformation_tracks_the_transformed_pet_on_owner() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(1233448, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), pet, 1233448),
        "native Dark Transformation must transform the actual controlled ghoul"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1235391),
        "the owner must track the actual transformed pet for the literal APL"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_single_target_uses_dark_transformation_and_putrefy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    engage(&mut run);
    advance_combat_observed(&mut run, |_| {}).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1233448)
                && event.kind == CombatLogKind::CastSuccess),
        "the literal ST rotation must use Dark Transformation"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1247378)
                && event.kind == CombatLogKind::CastSuccess),
        "the literal ST rotation must use Putrefy"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_raise_dead_produces_a_permanent_controlled_melee_pet() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("ordinary precombat Raise Dead must populate the native current-pet relation");
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        26125
    );
    assert_eq!(
        run.app
            .world()
            .get::<crate::hunter_pet::HunterPet>(pet)
            .unwrap()
            .owner,
        run.actor
    );
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..1300 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    assert!(
        crate::spell_cast::targets::is_alive(run.app.world(), pet),
        "the specialization ghoul is permanent, not the one-minute Frost/Blood guardian"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(pet.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "the real pet must reach and attack the selected hostile primary"
    );
}
