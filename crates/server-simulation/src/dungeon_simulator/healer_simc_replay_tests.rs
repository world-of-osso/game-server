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
