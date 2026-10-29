//! Harness-only Holy Light fixture, not a production Holy Paladin policy.
use super::{Policy, Registration, replay_with_party_registry, replay_with_registry};
use crate::dungeon_simulator::dps_parity::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use bevy::prelude::{Entity, World};
use shared::components::Health;
use std::path::Path;

#[test]
#[ignore = "requires canonical world database and healer-tank profiles"]
fn reference_healer_remains_alive_during_single_faction_setup() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    crate::spell_cast::cast_triggered(run.app.world_mut(), tank, 1308399, Some(tank), 1);
    let world = run.app.world_mut();
    assert!(
        world
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(world)
            .any(|summon| summon.owner == tank && summon.spell_id == 1308399),
        "real tank summon must be present before healer setup"
    );
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let world = run.app.world();
    assert_eq!(
        world.get::<shared::components::UnitFactionTemplate>(healer),
        world.get::<shared::components::UnitFactionTemplate>(tank)
    );
    assert!(
        world.get::<Health>(healer).unwrap().current > 0.0,
        "a same-party healer must survive all setup updates"
    );
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !events
            .iter()
            .any(|event| event.target == Some(healer.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage),
        "a tank summon must not attack the reference healer during setup"
    );
}

#[test]
#[ignore = "requires canonical world database and healer-tank profiles"]
fn beacon_transfers_healing_between_player_and_follower_party_members() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let party = crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    let follower = party.members[0];
    for (beacon, healed) in [(tank, follower), (follower, tank)] {
        for target in [beacon, healed] {
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * 0.25;
        }
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 53563, Some(beacon), 1);
        let before = run.app.world().get::<Health>(beacon).unwrap().current;
        let healed_before = run.app.world().get::<Health>(healed).unwrap().current;
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 19750, Some(healed), 1);
        let after = run.app.world().get::<Health>(beacon).unwrap().current;
        let effective = run.app.world().get::<Health>(healed).unwrap().current - healed_before;
        let transferred = after - before;
        assert!(effective > 0.0);
        assert!(
            (transferred - effective * 0.15).abs() <= 2.0,
            "Beacon {beacon:?} received {transferred} from {effective} healing on {healed:?}"
        );
    }
}

#[test]
#[ignore = "requires canonical world database and Preservation reference profile"]
fn preservation_lifebind_aggregates_raw_healing_on_native_party() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 1468)
            .unwrap();
    crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 360995, Some(tank), 1);
    assert!(crate::auras::has_aura(run.app.world(), healer, 373267));
    assert!(crate::auras::has_aura(run.app.world(), tank, 373267));
    for unit in [tank, healer] {
        let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = if unit == tank {
            health.max
        } else {
            health.max * 0.25
        };
    }
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..2 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 361509, Some(tank), 1);
    }
    for _ in 0..3 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    assert!(!events.iter().any(|event| event.spell_id == Some(373268)));
    for _ in 0..2 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let raw: i32 = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(361509)
                && event.kind == shared::protocol::CombatLogKind::Heal
                && event.target == Some(tank.to_bits())
        })
        .map(|event| {
            assert_eq!(
                event.amount, event.overflow,
                "source heal must be entirely overheal"
            );
            event.amount
        })
        .sum();
    assert!(raw > 0);
    let transfers: Vec<_> = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(373268)
                && event.kind == shared::protocol::CombatLogKind::Heal
                && event.target == Some(healer.to_bits())
        })
        .collect();
    assert_eq!(
        transfers.len(),
        1,
        "two heals must aggregate into one 200 ms transfer"
    );
    assert!((transfers[0].amount as f32 - raw as f32 * 0.6).abs() <= 2.0);
}

#[test]
#[ignore = "requires canonical world database and Preservation reference profile"]
fn preservation_lifebind_transfers_periodic_overhealing() {
    let mut run = crate::dungeon_simulator::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer =
        crate::dungeon_simulator::dps_parity::spawn_second_reference_player(&mut run, 1468)
            .unwrap();
    crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 360995, Some(tank), 1);
    for unit in [tank, healer] {
        let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = if unit == tank {
            health.max
        } else {
            health.max * 0.25
        };
    }
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 366155, Some(tank), 1);
    for _ in 0..45 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let tick = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(366155)
                && event.periodic
                && event.target == Some(tank.to_bits())
        })
        .expect("Reversion must tick on the bonded ally");
    assert_eq!(
        tick.amount, tick.overflow,
        "source tick must be fully overhealed"
    );
    let transfer = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(373268)
                && event.target == Some(healer.to_bits())
        })
        .expect("Lifebind must transfer the periodic heal");
    assert!((transfer.amount as f32 - tick.amount as f32 * 0.6).abs() <= 2.0);
}

struct FixturePolicy {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

fn prepare(
    run: &mut crate::dungeon_simulator::dps_parity::Run,
    healer: Entity,
) -> Result<Box<dyn Policy>, String> {
    let mut bindings = Bindings {
        actions: [("holy_light".into(), 82326)].into(),
        global_cooldown_spell: 82326,
        ..Default::default()
    };
    let equipment = Equipment::load(run.app.world_mut(), healer, &mut bindings)?;
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 12.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), healer, encounter)?;
    let program =
        server::simc_apl::Program::parse(&[("default", "holy_light,if=target.health.pct<95")])?;
    Ok(Box::new(FixturePolicy {
        driver: Driver::new(program),
        bindings,
        equipment,
        encounter,
    }))
}

impl Policy for FixturePolicy {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String> {
        let target = party
            .iter()
            .copied()
            .filter_map(|unit| {
                let health = world.get::<Health>(unit)?;
                (health.current > 0.0 && health.max > 0.0)
                    .then_some((unit, health.current / health.max))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(unit, _)| unit)
            .ok_or("no living party target")?;
        let targets = [target];
        let prepared = Prepared::read(
            world,
            healer,
            &targets,
            &self.equipment,
            [NativeAction {
                name: "holy_light",
                spell: 82326,
                talent: None,
                self_target: false,
            }],
            false,
        )?;
        let facts = NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
            .with_targets(Some(target), &targets);
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
        }
        Ok(())
    }
}

#[test]
#[ignore = "native replay fixture requires canonical world database"]
fn simc_party_pulse_hits_all_five_native_party_members() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dungeon_simulator/fixtures");
    let registry: &[Registration] = &[(65, prepare)];
    let report = replay_with_party_registry(
        65,
        &dir.join("healer_simc_fixture.log"),
        Some(&dir.join("healer_simc_party_fixture.log")),
        12,
        70448,
        registry,
    )
    .unwrap();
    assert_eq!(report["party_incoming_events"], 5);
    let party_damage = report["metrics"]["party_damage"].as_object().unwrap();
    assert_eq!(
        party_damage.len(),
        5,
        "every party member must take native damage: {report}"
    );
    // The tank can absorb its hit; the other four must take real health damage.
    assert!(
        party_damage
            .values()
            .filter(|v| v.as_u64().unwrap() > 0)
            .count()
            >= 4,
        "{report}"
    );
}

#[test]
#[ignore = "native replay fixture requires canonical world database"]
fn simc_healer_fixture_replays_damage_and_reports_receipts() {
    let trace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/dungeon_simulator/fixtures/healer_simc_fixture.log");
    assert_eq!(
        crate::dungeon_simulator::healer_replay::replay_simc(0, &trace, 12, 70448).unwrap_err(),
        "NATIVE_HEALER_POLICY_ABSENT:0"
    );
    let registry: &[Registration] = &[(65, prepare)];
    let report = replay_with_registry(65, &trace, 12, 70448, registry).unwrap();
    assert_eq!(report["incoming_events"], 3);
    assert_eq!(report["level"], 90);
    let metrics = &report["metrics"];
    let idle = metrics["idle_gcd_pct"].as_f64().unwrap();
    assert!(
        (0.0..100.0).contains(&idle),
        "cast fixture must contain both busy and idle time: {report}"
    );
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert!(
        metrics["spell_healing"]
            .as_object()
            .unwrap()
            .values()
            .any(|spell| spell["effective_hps"].as_f64().unwrap() > 0.0)
    );
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 13);
    println!("fixture-only Holy Light, not production policy: {report}");
}
