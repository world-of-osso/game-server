//! Actual equipped Call the Swarm/Void Eruption delivery, without forced proc rolls.
use super::*;
use crate::auras::{ActiveAuras, AuraSource};
use crate::followers::simulator::ParityActor;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::spell_info::SpellStore;
use shared::protocol::CombatLogKind;

const PAYLOADS: &[(u32, u32)] = &[(1310082, 1310088), (1310208, 1310209)];

fn prepare_equipped_probe(spec: u32) -> Run {
    let (profile, traits) = read_reference(spec).unwrap();
    let identity = ParityActor::Player {
        race: inputs::read_reference_race(&profile.race).unwrap(),
    };
    let driver = measurement_driver(spec, &identity).expect("native literal equipper driver");
    let mut run = prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    run.spec_driver = Some((driver.prepare)(&mut run).unwrap());
    run
}

fn assert_equipped_driver_budget(run: &Run, id: u32) -> f32 {
    let world = run.app.world();
    let driver = world
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == id && aura.caster == run.actor)
        .expect("actual profile must equip the native item driver");
    assert!(
        matches!(driver.source, AuraSource::Equipment(_)),
        "item ownership, not a granted proc aura"
    );
    let level = driver.cast_item_level.expect("native equipped item level");
    let mut native = world.resource::<SpellStore>().get(id).unwrap().clone();
    native.cast_item_level = Some(level);
    let effect = native
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let expected =
        crate::spell_cast::effects::modded_base_points(world, run.actor, &native, effect);
    let actual = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(
        expected > 0.0,
        "native item-scaled driver must own a positive damage budget"
    );
    assert!(
        (actual - expected).abs() < 0.001,
        "equipped driver{id} amount{actual} must match native item-level{level} budget{expected}"
    );
    let role = crate::item_proc_data::role_multiplier(world, run.actor, id).unwrap();
    assert!(role > 0.0);
    actual * role
}

#[test]
#[ignore = "requires native Arcane follower profile and full world catalog"]
fn follower_acidbrood_uses_item_budget_while_creature_damage_scales() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;

    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(62, 1).expect("equipped Arcane follower");
    let actor = run.actor;
    let target = run.targets[0];
    let world = run.app.world();
    assert!(
        world
            .get::<crate::followers::ClassCombatant>(actor)
            .is_some()
    );
    assert!(world.get::<shared::components::Npc>(actor).is_some());
    assert!(
        world
            .get_resource::<crate::level_scaling::CreatureSpellStats>()
            .is_some()
    );
    let budget = assert_equipped_driver_budget(&run, 1310082);

    // The target is a native creature, so the same zero-coefficient damage effect
    // must still take the full catalog's CreatureSpellDamage branch.
    let world = run.app.world();
    let spell = world.resource::<SpellStore>().get(1310088).unwrap();
    let mut effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .clone();
    effect.base_points = budget;
    let stats = world.resource::<crate::level_scaling::CreatureSpellStats>();
    let level = world
        .get::<shared::components::UnitLevel>(target)
        .unwrap()
        .0;
    let tuning = spell.misc.as_ref().map_or(0, |misc| misc.content_tuning_id);
    let auto_attack = spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.attributes[0] & 0x0008_0000 != 0);
    let creature_base =
        crate::spell_cast::effects::modded_base_points(world, target, spell, &effect);
    let expected_creature = (stats.evaluate(auto_attack, level, tuning) * budget / 100.0).round();
    assert!(
        (creature_base - expected_creature).abs() < 1.0,
        "creature={creature_base} expected={expected_creature}"
    );
    assert!(
        creature_base > budget * 2.0,
        "the control must exercise creature scaling"
    );

    run.app
        .insert_resource(FactorCapture::new(actor, target, 1310088, "acidbrood", 1));
    engage(&mut run);
    for _ in 0..2400 {
        drive_player(&mut run).unwrap();
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(event) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1310088)
        {
            let actual = event["factors"]["native_scaled_amount"].as_f64().unwrap();
            assert!(
                (actual - f64::from(budget)).abs() < 1.0,
                "Acidbrood base {actual}, item budget {budget}; {event}"
            );
            return;
        }
    }
    panic!("no real Acidbrood hit during 120 seconds");
}

#[test]
#[ignore = "requires exact native64/62/263 equipped profile, traits and world DB"]
fn native_equipped_midnight_item_damage_payloads_are_nonzero() {
    let _seed = crate::random::seed_scope(70448);
    let spec = std::env::var("DPS_PARITY_SPEC")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(matches!(spec, 64 | 62 | 263), "qualified native equippers");
    let mut run = prepare_equipped_probe(spec);
    let budgets: Vec<_> = PAYLOADS
        .iter()
        .map(|&(driver, damage)| (driver, damage, assert_equipped_driver_budget(&run, driver)))
        .collect();
    engage(&mut run);
    let (seconds, _, _) = advance_combat(&mut run).expect("ordinary native equipped combat");
    assert!(seconds >= 299.0, "full native proc opportunity window");
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let mut missing = Vec::new();
    for (driver, damage, budget) in budgets {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(damage)
                    && event.kind == CombatLogKind::Damage
            })
            .collect();
        let positive = hits.iter().filter(|event| event.amount > 0).count();
        eprintln!(
            "native item damage spec{spec} driver{driver} leaf{damage} budget{budget} hits{} positive{positive}",
            hits.len()
        );
        if positive == 0 {
            missing.push((driver, damage, hits.len(), budget));
        }
    }
    assert!(
        missing.is_empty(),
        "actual equipped native item payloads must deal their positive scaled damage: {missing:?}"
    );
}
