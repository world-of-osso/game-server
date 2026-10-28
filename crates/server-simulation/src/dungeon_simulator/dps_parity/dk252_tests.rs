use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_army_grants_forbidden_knowledge_and_summons_magus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1242223));
    let world = run.app.world_mut();
    assert!(
        world
            .query::<(
                &shared::components::Npc,
                &crate::spell_cast::summon::SummonedCreature
            )>()
            .iter(world)
            .any(|(npc, summon)| npc.template_id == 163366
                && summon.owner == run.actor
                && summon.spell_id == 317776)
    );
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_dark_transformation_summons_selected_magus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(1233448, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let world = run.app.world_mut();
    assert!(
        world
            .query::<(
                &shared::components::Npc,
                &crate::spell_cast::summon::SummonedCreature
            )>()
            .iter(world)
            .any(|(npc, summon)| npc.template_id == 163366
                && summon.owner == run.actor
                && summon.spell_id == 317776)
    );
}

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
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let riders: std::collections::HashSet<_> = {
        let world = run.app.world_mut();
        world
            .query::<(
                &shared::components::Npc,
                &crate::spell_cast::summon::SummonedCreature,
            )>()
            .iter(world)
            .filter_map(|(npc, summon)| {
                (summon.owner == run.actor
                    && [444248, 444251, 444252, 444254].contains(&summon.spell_id))
                .then_some(npc.template_id)
            })
            .collect()
    };
    assert_eq!(
        riders,
        [221632, 221633, 221634, 221635].into_iter().collect(),
        "selected Apocalypse Now must summon all four actual native Horsemen"
    );
}

#[test]
#[ignore = "requires private native252 Rider templates/profile and player actor"]
fn dk252_army_riders_melee_and_expire_at_selected_duration() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    let started = crate::auras::aura_now(run.app.world());
    let duration =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 444040, 1)
            as f64
            / 1000.0;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let riders: Vec<_> = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .filter_map(|(entity, summon)| {
                (summon.owner == run.actor
                    && [444248, 444251, 444252, 444254].contains(&summon.spell_id))
                .then_some((entity, summon.expires_at))
            })
            .collect()
    };
    assert_eq!(riders.len(), 4, "the actual four Horsemen must be present");
    for (_, expiry) in &riders {
        assert!(
            (expiry.unwrap() - started - duration).abs() < 0.2,
            "Horseman lifetime must use selected Apocalypse Now native duration"
        );
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for (rider, _) in &riders {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(rider.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "each actual Horseman must melee the selected target"
        );
    }
    while crate::auras::aura_now(run.app.world()) < started + duration + 1.0 {
        run.app.update();
    }
    assert!(
        riders
            .iter()
            .all(|(rider, _)| run.app.world().get_entity(*rider).is_err()),
        "Horsemen must expire after their selected native duration"
    );
}

#[test]
#[ignore = "requires private native252 Rider templates/profile and player actor"]
fn dk252_horsemen_melee_inherits_live_owner_attack_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .attack_power += 10_000.0;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let riders: Vec<_> = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .filter_map(|(entity, summon)| {
                (summon.owner == run.actor
                    && [444248, 444251, 444252, 444254].contains(&summon.spell_id))
                .then_some(entity)
            })
            .collect()
    };
    assert_eq!(riders.len(), 4, "four real Horsemen must be present");
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for rider in riders {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(rider.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .map(|event| event.amount)
            .collect();
        assert!(
            hits.iter().any(|hit| *hit > 100),
            "each actual Horseman must inherit live owner melee power: {hits:?}"
        );
    }
}

#[test]
#[ignore = "requires private native252 Rider templates/profile and player actor"]
fn dk252_horsemen_use_their_native_damage_actions() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    for _ in 0..300 {
        run.app.update();
    }
    let riders: std::collections::HashMap<_, _> = {
        let world = run.app.world_mut();
        world
            .query::<(
                Entity,
                &shared::components::Npc,
                &crate::spell_cast::summon::SummonedCreature,
            )>()
            .iter(world)
            .filter_map(|(entity, npc, summon)| {
                (summon.owner == run.actor
                    && [444248, 444251, 444252, 444254].contains(&summon.spell_id))
                .then_some((npc.template_id, entity.to_bits()))
            })
            .collect()
    };
    assert_eq!(riders.len(), 4, "four real Horsemen must be present");
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for (entry, spell) in [
        (221632, 445504),
        (221633, 445513),
        (221634, 445508),
        (221635, 445507),
    ] {
        let actor = riders[&entry];
        assert!(
            events.iter().any(|event| event.source == Some(actor)
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
            "real Horseman {entry} must deal damage with native spell {spell}"
        );
    }
}

#[test]
#[ignore = "requires private native252 Rider templates/profile and player actor"]
fn dk252_horseman_spell_inherits_owner_melee_power() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    for _ in 0..60 {
        run.app.update();
    }
    let rider = {
        let world = run.app.world_mut();
        world
            .query::<(
                Entity,
                &shared::components::Npc,
                &crate::spell_cast::summon::SummonedCreature,
            )>()
            .iter(world)
            .find_map(|(entity, npc, summon)| {
                (summon.owner == run.actor && npc.template_id == 221633).then_some(entity)
            })
            .expect("actual Whitemane must exist")
    };
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .attack_power += 10_000.0;
    let world = run.app.world();
    let expected = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .total_attack_power(
            crate::stats::WeaponAttackType::MainHand,
            world.get::<crate::stats::StatModifiers>(run.actor),
        )
        * 0.70125;
    run.app.insert_resource(FactorCapture::new(
        rider,
        run.targets[0],
        445513,
        "horseman_coil",
        1,
    ));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        rider,
        445513,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..100 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(power) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 445513)
            .and_then(|event| event["factors"]["a_power"].as_f64())
        {
            assert!(
                (power - f64::from(expected)).abs() < 1.0,
                "actual Horseman spell must inherit owner melee AP: actual={power}, expected={expected}"
            );
            return;
        }
    }
    panic!("the actual Horseman must land native Death Coil");
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_lesser_ghoul_expiry_triggers_ruptured_viscera() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        275430,
        Some(run.actor),
        1
    ));
    for _ in 0..300 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|e| e.spell_id == Some(1247379)
            && e.target == Some(run.targets[0].to_bits())
            && e.kind == CombatLogKind::Damage
            && e.amount > 0),
        "expired real ghoul must deal native Ruptured Viscera damage"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_scourge_strike_spends_ready_ghoul_and_summons_it() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(85948, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let ready = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1254252 && aura.caster == run.actor)
            .map_or(0, |aura| aura.stacks)
    };
    let initial = ready(&run);
    assert!(
        initial > 0,
        "actual Festering Strike must earn ready ghouls"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(55090, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        ready(&run),
        initial - 1,
        "Scourge Strike must spend one earned ready ghoul"
    );
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(275430)
                && event.kind == CombatLogKind::Summon),
        "the spent ready ghoul must become an actual native lesser ghoul"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_scourge_strike_commands_active_army_ghouls() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let army: std::collections::HashSet<_> = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .filter_map(|(entity, summon)| {
                (summon.owner == run.actor && summon.spell_id == 1294026)
                    .then_some(entity.to_bits())
            })
            .collect()
    };
    assert_eq!(
        army.len(),
        8,
        "actual Army must be present before the command"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(85948, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1254252));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(55090, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let commanded: std::collections::HashSet<_> = events
        .iter()
        .filter_map(|event| {
            (event.spell_id == Some(1294264)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(run.targets[0].to_bits())
                && event.amount > 0)
                .then_some(event.source)
                .flatten()
                .filter(|source| army.contains(source))
        })
        .collect();
    assert_eq!(
        commanded, army,
        "each actual Army ghoul must execute native Death Order"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_army_death_order_inherits_owner_melee_power() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let ghoul = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .find_map(|(entity, summon)| {
                (summon.owner == run.actor && summon.spell_id == 1294026).then_some(entity)
            })
            .expect("actual Army ghoul must exist")
    };
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .attack_power += 10_000.0;
    let world = run.app.world();
    let expected = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .total_attack_power(
            crate::stats::WeaponAttackType::MainHand,
            world.get::<crate::stats::StatModifiers>(run.actor),
        )
        * 0.180431625
        * 1.4
        * (1.0
            + crate::spell_cast::scripts::script_effect_value(world, run.actor, 1265971, 4)
                / 100.0);
    run.app.insert_resource(FactorCapture::new(
        ghoul,
        run.targets[0],
        1294264,
        "death_order",
        1,
    ));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        ghoul,
        1294264,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..100 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(power) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1294264)
            .and_then(|event| event["factors"]["a_power"].as_f64())
        {
            assert!(
                (power - f64::from(expected)).abs() < 1.0,
                "actual Army order must inherit selected owner melee AP: actual={power}, expected={expected}"
            );
            return;
        }
    }
    panic!("the actual Army ghoul must land Death Order");
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_scourge_strike_commands_army_epidemic_on_three_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 3).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let army: std::collections::HashSet<_> = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .filter_map(|(entity, summon)| {
                (summon.owner == run.actor && summon.spell_id == 1294026)
                    .then_some(entity.to_bits())
            })
            .collect()
    };
    assert_eq!(
        army.len(),
        8,
        "actual Army must be present before the command"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(85948, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(55090, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let targets: std::collections::HashSet<_> = events
        .iter()
        .filter_map(|event| {
            (event.spell_id == Some(1300330)
                && event.kind == CombatLogKind::Damage
                && event.source.is_some_and(|source| army.contains(&source))
                && event.amount > 0)
                .then_some(event.target)
                .flatten()
        })
        .collect();
    assert!(
        run.targets
            .iter()
            .all(|target| targets.contains(&target.to_bits())),
        "actual Army Epidemic Orders must damage all three targets: {targets:?}"
    );
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_scourge_ghoul_reduces_putrefy_recharge() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
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
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: 1254252,
            effect_mask: 3,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let bindings = Bindings {
        actions: [("putrefy".into(), 1247378)].into(),
        ..Default::default()
    };
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let read = |run: &Run| {
        NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
            .value("cooldown.putrefy.full_recharge_time")
            .unwrap()
    };
    let before = read(&run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(55090, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        read(&run) < before - 2.0,
        "earned ghoul must reduce Putrefy recharge"
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_reaping_grants_buff_and_increases_execute_coil_damage() {
    fn coil_damage(health_fraction: f32) -> i64 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(252, 1).unwrap();
        engage(&mut run);
        {
            let mut hp = run
                .app
                .world_mut()
                .get_mut::<shared::components::Health>(run.targets[0])
                .unwrap();
            hp.current = hp.max * health_fraction;
        }
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            47632,
            Some(run.targets[0]),
            1
        ));
        for _ in 0..30 {
            run.app.update();
        }
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(47632)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("native Death Coil child must hit")
            .amount
            .into()
    }
    let above = coil_damage(0.9);
    let below = coil_damage(0.2);
    assert!(
        below > above,
        "selected Reaping must increase execute damage: {above} vs {below}"
    );
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(1233448, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1242654));
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_lesser_ghoul_uses_claw_then_transformed_sweeping_claws() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        1294026,
        Some(run.actor),
        1
    ));
    for _ in 0..160 {
        run.app.update();
    }
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: 1235391,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..160 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for spell in [199373, 1278150] {
        assert!(
            events.iter().any(|e| e.spell_id == Some(spell)
                && e.kind == CombatLogKind::Damage
                && e.amount > 0),
            "actual lesser ghoul must damage with native {spell}"
        );
    }
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_trollbane_chains_then_scourge_strike_shatters_three_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 3).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None)).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..200 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), run.targets[0], 444826) { break; }
    }
    assert!(crate::auras::has_aura(run.app.world(), run.targets[0], 444826), "real Trollbane must apply native Chains of Ice");
    literal_rotation::execution::execute(run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(55090, run.targets[0], Some(run.targets[0]))).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..20 { run.app.update(); }
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    for target in &run.targets {
        assert!(events.iter().any(|event| event.source == Some(run.actor.to_bits())
            && event.target == Some(target.to_bits()) && event.spell_id == Some(444834)
            && event.kind == CombatLogKind::Damage && event.amount > 0),
            "Scourge Strike must make native Icy Fury hit {target:?}");
    }
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_army_summons_abomination_with_cloud_and_melee() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let actor = {
        let world = run.app.world_mut();
        world
            .query::<(
                Entity,
                &shared::components::Npc,
                &crate::spell_cast::summon::SummonedCreature,
            )>()
            .iter(world)
            .find_map(|(entity, npc, summon)| {
                (npc.template_id == 149555
                    && summon.owner == run.actor
                    && summon.spell_id == 288853)
                    .then_some(entity)
            })
            .expect("Army must summon its selected native Abomination")
    };
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for spell in [None, Some(1244347)] {
        assert!(
            events.iter().any(|e| e.source == Some(actor.to_bits())
                && e.spell_id == spell
                && e.kind == CombatLogKind::Damage
                && e.amount > 0),
            "actual Abomination must deal {spell:?} damage"
        );
    }
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_abomination_cloud_and_melee_inherit_live_owner_crit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..400 {
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .crit_pct = 100.0;
        run.app.update();
    }
    let actor = {
        let world = run.app.world_mut();
        world
            .query::<(
                Entity,
                &shared::components::Npc,
                &crate::spell_cast::summon::SummonedCreature,
            )>()
            .iter(world)
            .find_map(|(entity, npc, summon)| {
                (npc.template_id == 149555
                    && summon.owner == run.actor
                    && summon.spell_id == 288853)
                    .then_some(entity)
            })
            .expect("Army must summon its selected Abomination")
    };
    assert_eq!(
        crate::combat::read_pet_owner_critical_chance(run.app.world(), actor),
        Some(100.0)
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for spell in [None, Some(1244347)] {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.spell_id == spell
                    && event.kind == CombatLogKind::Damage
                    && event.crit
                    && event.amount > 0),
            "Abomination {spell:?} must critically hit using its owner's live crit"
        );
    }
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_grave_mastery_increases_actual_magus_critical_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(1247378, run.targets[0], Some(run.targets[0])),
    ).unwrap();
    let magus = (0..100).find_map(|_| {
        run.app.update();
        let world = run.app.world_mut();
        world.query::<(Entity, &shared::components::Npc)>().iter(world)
            .find_map(|(entity, npc)| (npc.template_id == 163366).then_some(entity))
    }).expect("Putrefy must summon an actual Magus");
    assert!(crate::auras::has_aura(run.app.world(), magus, 1238902));
    run.app.world_mut().get_mut::<crate::stats::CharacterStats>(run.actor).unwrap().crit_pct = 100.0;
    crate::auras::remove_auras(run.app.world_mut(), magus, crate::auras::RemoveReason::Cancelled, |aura| aura.spell_id == 1238902);
    let target = run.targets[0];
    let hit = |run: &mut Run| {
        run.app.world_mut().resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.clear();
        assert!(crate::spell_cast::try_cast_triggered(run.app.world_mut(), magus, 1297055, Some(target), 1));
        for _ in 0..40 {
            run.app.update();
            if let Some(amount) = run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter()
                .find(|event| event.source == Some(magus.to_bits()) && event.spell_id == Some(1297055)
                    && event.kind == CombatLogKind::Damage && event.crit).map(|event| event.amount) {
                return amount;
            }
        }
        panic!("actual Magus bolt must land and crit");
    };
    let without = hit(&mut run);
    assert!(crate::spell_cast::try_cast_triggered(run.app.world_mut(), magus, 1238902, Some(magus), 1));
    let with = hit(&mut run);
    let expected = 1.0 + crate::spell_cast::scripts::script_effect_value(run.app.world(), magus, 1238902, 0) / 100.0;
    assert!((with as f32 / without as f32 - expected).abs() < 0.01,
        "native Grave Mastery must increase the real Magus critical hit: {without} -> {with}");
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_putrefy_triggers_selected_apocalypse_rune() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 327087));
    for _ in 0..20 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            390220,
            Some(run.targets[0]),
            1
        ));
    }
    for _ in 0..120 {
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
            .any(|e| e.spell_id == Some(327093) && e.kind == CombatLogKind::Damage && e.amount > 0),
        "selected Rune of the Apocalypse must produce native Pestilence on Putrefy impacts"
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_sudden_doom_death_coil_summons_doomed_bidding_ghoul() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: 81340,
            effect_mask: 15,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        run.actor,
        shared::components::PowerType::RunicPower,
        1000.0,
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(47541, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    let world = run.app.world_mut();
    assert!(
        world
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(world)
            .any(|summon| summon.owner == run.actor && summon.spell_id == 275430),
        "Sudden Doom Death Coil must summon a real Doomed Bidding ghoul"
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_ghoul_attacks_earn_infected_claws() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    for _ in 0..5 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            1294026,
            Some(run.actor),
            1
        ));
    }
    for _ in 0..400 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|e| e.spell_id == Some(1241786)
            && e.target == Some(run.targets[0].to_bits())
            && e.kind == CombatLogKind::Damage
            && e.amount > 0),
        "actual ghoul hits must earn Infected Claws"
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_runic_spends_trigger_runic_corruption() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    let mut earned = false;
    for _ in 0..10 {
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            run.actor,
            shared::components::PowerType::RunicPower,
            1000.0,
        );
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                47541,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        earned |= crate::auras::has_aura(run.app.world(), run.actor, 51460);
        for _ in 0..40 {
            run.app.update();
        }
    }
    assert!(earned, "real Runic Power spends must earn Runic Corruption");
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_abomination_critical_cloud_tracks_live_owner_mastery() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let abomination = {
        let world = run.app.world_mut();
        world
            .query::<(
                Entity,
                &shared::components::Npc,
                &crate::spell_cast::summon::SummonedCreature,
            )>()
            .iter(world)
            .find_map(|(actor, npc, summon)| {
                (npc.template_id == 149555
                    && summon.owner == run.actor
                    && summon.spell_id == 288853)
                    .then_some(actor)
            })
            .expect("Army must summon its actual Abomination")
    };
    run.app
        .world_mut()
        .entity_mut(abomination)
        .insert(crate::stats::CharacterStats {
            crit_pct: 100.0,
            ..Default::default()
        });
    let hit = |run: &mut Run| {
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .crit_pct = 100.0;
        run.app
            .world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            abomination,
            1244347,
            Some(abomination),
            1
        ));
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(abomination.to_bits())
                    && event.spell_id == Some(1244347)
                    && event.kind == CombatLogKind::Damage
                    && event.crit
            })
            .expect("actual Abomination cloud must crit")
            .amount
    };
    let before_mastery = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .mastery_pct;
    let before = hit(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .mastery_pct = before_mastery + 50.0;
    run.app.update();
    let after_mastery = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .mastery_pct;
    assert!(
        after_mastery > before_mastery,
        "fixture must change the owner's mastery"
    );
    let after = hit(&mut run);
    let effect = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(77515)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 5)
        .unwrap();
    let expected = (100.0 + effect.base_points + effect.bonus_coefficient * after_mastery)
        / (100.0 + effect.base_points + effect.bonus_coefficient * before_mastery);
    assert!(
        (after as f32 / before as f32 - expected).abs() < 0.01,
        "actual Abomination critical cloud must follow the native live mastery multiplier: {before} -> {after}, expected ratio {expected}"
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_soul_reaper_spends_ready_ghouls() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    for _ in 0..3 {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 1254252,
                effect_mask: 3,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    {
        let mut hp = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(run.targets[0])
            .unwrap();
        hp.current = hp.max * 0.2;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(343294, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1254252));
    let world = run.app.world_mut();
    let count = world
        .query::<&crate::spell_cast::summon::SummonedCreature>()
        .iter(world)
        .filter(|summon| summon.owner == run.actor && summon.spell_id == 275430)
        .count();
    assert_eq!(
        count, 3,
        "Soul Reaper must turn three earned stacks into actual ghouls"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_lord_bolt_scales_with_consumed_magus_remaining_time() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;

    fn first_bolt_multiplier(remaining: f64) -> f64 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(252, 1).unwrap();
        engage(&mut run);
        for index in 0..3 {
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                run.actor,
                1242294,
                Some(run.actor),
                1,
            ));
            run.app.update();
            if index < 2 {
                let world = run.app.world_mut();
                let now = crate::auras::aura_now(world);
                let magi: Vec<_> = world
                    .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
                    .iter(world)
                    .filter(|(_, summon)| summon.owner == run.actor && summon.spell_id == 1242294)
                    .map(|(entity, _)| entity)
                    .collect();
                assert_eq!(magi.len(), index + 1, "the native Magi must exist");
                for magus in magi {
                    world
                        .get_mut::<crate::spell_cast::summon::SummonedCreature>(magus)
                        .unwrap()
                        .expires_at = Some(now + remaining);
                }
            }
        }
        let lord = {
            let world = run.app.world_mut();
            world
                .query::<(Entity, &shared::components::Npc)>()
                .iter(world)
                .find_map(|(entity, npc)| (npc.template_id == 264321).then_some(entity))
                .expect("three actual Magi must create the native Lord")
        };
        run.app.insert_resource(FactorCapture::new(
            lord,
            run.targets[0],
            1297086,
            "lord_bolt",
            1,
        ));
        for _ in 0..200 {
            run.app.update();
            let capture = run.app.world().resource::<FactorCapture>().export();
            if let Some(multiplier) = capture["events"]
                .as_array()
                .unwrap()
                .iter()
                .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1297086)
                .and_then(|event| event["factors"]["mult"].as_f64())
            {
                return multiplier;
            }
        }
        panic!("the real Lord must cast its native Necrotic Bolt");
    }

    let short = first_bolt_multiplier(2.0);
    let long = first_bolt_multiplier(14.0);
    assert!(
        long > short * 1.1,
        "longer consumed Magus lifetime must increase actual Lord damage: short={short}, long={long}"
    );
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_blightburst_applies_and_extends_owned_diseases() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    for (impact, plague) in [(1277016, 1240996), (390220, 191587)] {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            impact,
            Some(run.targets[0]),
            1
        ));
        let expiry = |run: &Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(run.targets[0])
                .unwrap()
                .iter()
                .find(|a| a.caster == run.actor && a.spell_id == plague)
                .unwrap()
                .expires_at
                .unwrap()
        };
        let before = expiry(&run);
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            impact,
            Some(run.targets[0]),
            1
        ));
        let after = expiry(&run);
        assert!(
            after - before >= 2.99,
            "Blightburst must extend disease {plague}"
        );
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for eruption in [1241171, 1241167] {
        assert!(
            events.iter().any(|e| e.spell_id == Some(eruption)
                && e.kind == CombatLogKind::Damage
                && e.amount > 0),
            "Blightburst must erupt active disease using native child {eruption}"
        );
    }
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_lord_magus_uses_selected_native_lifetime() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    // Forbidden Knowledge can otherwise summon a third Magus and consume this one.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.targets[0],
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1240996,
    );
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
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.targets[0],
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1240996,
        );
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_death_coil_deposits_rolling_devastation() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        47632,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..30 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.targets[0])
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 390271 && a.caster == run.actor)
    );
    for _ in 0..120 {
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
            .any(|e| e.spell_id == Some(390271) && e.kind == CombatLogKind::Damage && e.amount > 0),
        "Coil of Devastation must tick after actual Death Coil damage"
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_death_coil_extends_real_dark_transformation() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(1233448, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let expiry = |run: &Run, entity, spell| {
        let auras = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(entity)
            .expect("entity aura list");
        let aura = auras
            .iter()
            .find(|a| a.caster == run.actor && a.spell_id == spell)
            .unwrap_or_else(|| {
                panic!(
                    "missing aura {spell} on {entity:?}: {:?}",
                    auras
                        .iter()
                        .map(|a| (a.spell_id, a.caster))
                        .collect::<Vec<_>>()
                )
            });
        aura.expires_at
            .unwrap_or_else(|| panic!("aura {spell} on {entity:?} has no expiry"))
    };
    let owner_before = expiry(&run, run.actor, 1235391);
    let pet_before = expiry(&run, pet, 1233448);
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        run.actor,
        shared::components::PowerType::RunicPower,
        1000.0,
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(47541, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(expiry(&run, run.actor, 1235391) >= owner_before + 0.99);
    assert!(expiry(&run, pet, 1233448) >= pet_before + 0.99);
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_transformed_permanent_ghoul_uses_sweeping_claws_on_three_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 3).unwrap();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    crate::combat::start_attack_now(
        run.app.world_mut(),
        run.actor,
        crate::combat::AutoAttack::new(run.targets[0]),
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(1233448, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(pet.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(91778)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "the transformed controlled ghoul must hit target {target:?} with native Sweeping Claws; pet={pet:?}, events={:?}",
            events
                .iter()
                .filter(|event| event.spell_id == Some(91778))
                .collect::<Vec<_>>()
        );
    }
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_whitemane_applies_and_grows_real_undeath() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let stacks = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.targets[0])
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 444633 && aura.caster == run.actor)
                    .map(|aura| aura.stacks)
            })
    };
    for _ in 0..200 {
        run.app.update();
        if stacks(&run).is_some() {
            break;
        }
    }
    let before = stacks(&run).expect("the real Whitemane must apply owner Undeath");
    for _ in 0..100 {
        run.app.update();
    }
    let after = stacks(&run).expect("Undeath must remain active");
    assert!(
        after > before,
        "actual periodic ticks must grow Undeath stacks: {before} -> {after}"
    );
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(444633)
                && event.periodic
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "owner Undeath must deal real periodic damage"
    );
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_literal_rotation_observes_sudden_doom_reaction() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    engage(&mut run);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: 81340,
            effect_mask: 15,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..50 {
        drive_player(&mut run).expect("literal Unholy must handle earned Sudden Doom reaction");
        run.app.update();
    }
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
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_necrotic_coil_hits_with_physical_and_shadow_children() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 3).unwrap();
    engage(&mut run);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        1242174,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for spell in [1242172, 1242178] {
        let hits = events
            .iter()
            .filter(|e| {
                e.source == Some(run.actor.to_bits())
                    && e.spell_id == Some(spell)
                    && e.kind == CombatLogKind::Damage
                    && e.amount > 0
            })
            .count();
        assert_eq!(
            hits, 3,
            "Necrotic Coil must hit each target once with child {spell}"
        );
    }
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

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_rune_spending_can_summon_a_ten_second_real_horseman() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    for _ in 0..30 {
        crate::spell_cast::powers::change_power(run.app.world_mut(), run.actor, shared::components::PowerType::Runes, 6.0);
        literal_rotation::execution::execute(run.app.world_mut(), run.actor,
            literal_rotation::execution::Execution::Spell(55090, run.targets[0], Some(run.targets[0]))).unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let now = crate::auras::aura_now(run.app.world());
        let found = {
            let world = run.app.world_mut();
            world.query::<&crate::spell_cast::summon::SummonedCreature>().iter(world)
                .find_map(|summon| (summon.owner == run.actor && [444248,444251,444252,444254].contains(&summon.spell_id))
                    .then_some(summon.expires_at)).flatten()
        };
        if let Some(expiry) = found {
            let duration = run.app.world().resource::<crate::spell_info::SpellStore>().get(454389).unwrap()
                .misc.as_ref().unwrap().duration.unwrap().duration_ms as f64 / 1000.0;
            assert!(expiry > now && expiry - now <= duration, "real proc rider must use its native temporary lifetime");
            return;
        }
        for _ in 0..40 { run.app.update(); }
    }
    panic!("actual rune spenders must earn a Rider's Champion summon");
}

#[test]
#[ignore = "requires native252 profile/traits and player actor"]
fn dk252_dread_plague_ticks_produce_sudden_doom() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 49530));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        1240996,
        Some(run.targets[0]),
        1,
    ));
    let mut earned = false;
    for _ in 0..600 {
        run.app.update();
        earned |= crate::auras::has_aura(run.app.world(), run.actor, 81340);
    }
    assert!(earned, "Dread Plague ticks must earn selected Sudden Doom");
}
