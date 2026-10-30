use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;
use std::sync::{Arc, Mutex};

#[test]
#[ignore = "requires native Blood, Frost and Unholy profiles"]
fn icy_talons_increases_swing_speed_without_increasing_swing_damage() {
    use shared::protocol::CombatLogKind;
    fn sample(spec: u32, stacked: bool) -> (i32, f32) {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_registered_player(spec, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        let world = run.app.world_mut();
        crate::auras::remove_auras(world, actor, crate::auras::RemoveReason::Cancelled, |a| {
            a.spell_id == 194879
        });
        world
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = 0.0;
        if stacked {
            assert!(crate::spell_cast::cast_triggered_with_overrides(
                world,
                actor,
                194879,
                Some(actor),
                1,
                crate::spell_cast::TriggeredSpellOverrides {
                    effect0_amount: Some(6.0),
                    aura_stacks: Some(3),
                    ..Default::default()
                }
            ));
        }
        run.app.update();
        let world = run.app.world_mut();
        let speed = world
            .get::<crate::auras::AuraStatMods>(actor)
            .map_or(1.0, |mods| {
                mods.attack_time_mult * mods.melee_ranged_attack_time_mult
            });
        let start = world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        let _hit_seed = crate::random::seed_scope(123);
        crate::combat::start_attack_now(world, actor, crate::combat::AutoAttack::new(target));
        for _ in 0..10 {
            run.app.update();
        }
        let hit = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..]
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
            })
            .expect("real melee swing")
            .amount;
        (hit, speed)
    }
    for spec in [250, 251, 252] {
        let (base, base_speed) = sample(spec, false);
        let (stacked, stacked_speed) = sample(spec, true);
        assert_eq!(
            base, stacked,
            "Icy Talons must not change per-swing damage for {spec}"
        );
        assert!(
            stacked_speed < base_speed,
            "Icy Talons must increase swing speed for {spec}: {base_speed} -> {stacked_speed}"
        );
    }
}

struct TargetFactsProbe(Arc<Mutex<Vec<(f64, f64)>>>);

impl SpecDriver for TargetFactsProbe {
    fn label(&self) -> &'static str {
        "tank_fixture_target_facts_probe"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let bindings = Bindings::default();
        let facts = NativeFacts::new(
            world,
            actor,
            &bindings,
            Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: FightStyle::Patchwerk,
            },
            targets.len(),
        )
        .with_targets(targets.first().copied(), targets);
        let remaining = facts.value("target.time_to_die")?;
        let boss = facts.value("target.is_boss")?;
        self.0.lock().unwrap().push((remaining, boss));
        Ok(())
    }
}

#[test]
#[ignore = "requires the matching MID2 Guardian profile and SimC DPS trace"]
fn tank_dps_replay_uses_reference_incoming_and_all_targets() {
    let directory = std::env::var("DPS_PARITY_TANK_TRACE_DIR").unwrap();
    let trace = std::path::Path::new(&directory).join("104.3.70448.simc.log");
    let report =
        super::super::tank_replay::replay_simc_with_targets(104, &trace, 3, 300, 70448, true, 1.0)
            .unwrap();
    assert_eq!(report["tank_dummy_scenario"]["attacker_level"], 93);
    assert_eq!(report["tank_dummy_scenario"]["armor_constant"], 3430.0);
    assert!(report["time_to_death"].is_null(), "{report}");
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(report["external_healing_effective"].as_f64().unwrap() > 0.0);
    let targets = report["combat"]["damage_by_target"].as_array().unwrap();
    assert_eq!(targets.len(), 21, "all ten add waves must be recorded");
    assert!(
        targets
            .iter()
            .all(|target| target["damage"].as_u64().unwrap() > 0)
    );
    assert_eq!(report["target_waves"].as_array().unwrap().len(), 10);
    assert!(
        report["melee_rage"]["taken"].as_f64().unwrap_or(0.0) > 0.0,
        "incoming attacks must grant Rage: {report}"
    );
}

#[test]
#[ignore = "uses the real MID2 player and tank replay target"]
fn tank_replay_drives_target_lifetime_and_boss_facts() {
    let mut run = prepare_registered_player(66, 1).unwrap();
    let observations = Arc::new(Mutex::new(Vec::new()));
    run.spec_driver = Some(Box::new(TargetFactsProbe(observations.clone())));
    super::super::tank_replay::replay_registered_run(run, &[], 2, &[]).unwrap();
    let observations = observations.lock().unwrap();
    let first = observations.first().unwrap();
    let last = observations.last().unwrap();
    assert!((first.0 - 300.0).abs() < 0.1, "{first:?}");
    assert!((last.0 - 298.05).abs() < 0.1, "{last:?}");
    assert!(observations.iter().all(|(_, boss)| *boss == 1.0));
}
