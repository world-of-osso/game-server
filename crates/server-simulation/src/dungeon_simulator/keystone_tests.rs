use super::*;

#[test]
fn keystone_progress_reports_running_state_on_simulated_minute_boundaries() {
    use shared::protocol::{ChallengeBoss, ChallengePhase, ChallengeRun, KeystoneInfo};

    let mut snapshot = ChallengeRun {
        run_id: 1,
        map_id: 2813,
        instance_id: 2,
        name: "Murder Row".into(),
        key: KeystoneInfo {
            item_guid: 1,
            item_id: 180653,
            challenge_id: 587,
            level: 2,
            affixes: vec![],
        },
        phase: ChallengePhase::Countdown,
        countdown_ms: 10_000,
        elapsed_ms: 0,
        time_limit_ms: 2_040_000,
        deaths: 0,
        time_lost_ms: 0,
        forces: 0,
        forces_required: 655,
        bosses: vec![
            ChallengeBoss {
                encounter_id: 3101,
                name: "Kystia".into(),
                defeated: false,
            },
            ChallengeBoss {
                encounter_id: 3102,
                name: "Zaen".into(),
                defeated: false,
            },
        ],
        upgrade_levels: 0,
        resulting_level: None,
        rating: None,
    };
    let route = [
        super::super::graph::RouteStep {
            pack: 10,
            anchor: [0.0; 3],
        },
        super::super::graph::RouteStep {
            pack: 8_100_243_029,
            anchor: [0.0; 3],
        },
    ];
    let mut reporter = Progress::default();
    let mut output = Vec::new();
    let idle = crate::creature_ai::CreatureState::Idle;
    let mut world = World::new();
    let follow = crate::creature_ai::FollowOwner::new(world.spawn_empty().id());
    let tank = TankProgress {
        position: Some([1.0, 2.0, 3.0]),
        state: Some(&idle),
        follow: Some(&follow),
    };
    reporter
        .write(&mut output, 0, &snapshot, None, &route, tank)
        .unwrap();
    assert!(output.is_empty(), "countdown is not running time");
    snapshot.phase = ChallengePhase::Running;
    reporter
        .write(&mut output, 0, &snapshot, None, &route, tank)
        .unwrap();
    reporter
        .write(&mut output, 59_999, &snapshot, Some(10), &route, tank)
        .unwrap();
    snapshot.forces = 75;
    snapshot.deaths = 1;
    snapshot.bosses[0].defeated = true;
    reporter
        .write(
            &mut output,
            60_000,
            &snapshot,
            Some(8_100_243_029),
            &route,
            tank,
        )
        .unwrap();
    reporter
        .write(
            &mut output,
            60_050,
            &snapshot,
            Some(8_100_243_029),
            &route,
            tank,
        )
        .unwrap();
    let output = String::from_utf8(output).unwrap();
    assert_eq!(
        output,
        format!(
            concat!(
                "KEYSTONE_PROGRESS sim_ms=0 forces=0/655 bosses=0/2 deaths=0 current_pack=None route_step=None/2 tank_position=Some([1.0, 2.0, 3.0]) tank_state=Some(Idle) tank_follow=Some({follow:?})\n",
                "KEYSTONE_PROGRESS sim_ms=60000 forces=75/655 bosses=1/2 deaths=1 current_pack=Some(8100243029) route_step=Some(2)/2 tank_position=Some([1.0, 2.0, 3.0]) tank_state=Some(Idle) tank_follow=Some({follow:?})\n",
            ),
            follow = follow,
        )
    );
    assert!(output.contains("moving: false"));
}

#[test]
fn parses_explicit_dungeon_key_and_five_spec_party() {
    let options = Options::parse("2515", "2", "66,105,262,63,253").unwrap();
    assert_eq!(
        (options.map_id, options.level, options.specs),
        (2515, 2, [66, 105, 262, 63, 253])
    );
    assert!(Options::parse("2515", "0", "66,105,262,63,253").is_err());
    assert!(Options::parse("2515", "2", "66,105").is_err());
    assert!(Options::parse("bad", "2", "66,105,262,63,253").is_err());
}

#[test]
fn unsupported_dungeon_produces_an_explicit_incomplete_record() {
    let graph = super::super::graph::read_graph().unwrap();
    let options = Options {
        map_id: graph.map_id + 1,
        level: 2,
        specs: [66, 105, 262, 63, 253],
    };
    let record = super::super::campaign::run_keystone(7, options);
    assert_eq!(record.outcome.status, "aborted");
    assert!(record.outcome.reason.contains("does not match graph"));
    assert_eq!(record.map_id, options.map_id);
    assert_eq!(record.requested_specs, options.specs);
    assert!(record.route.is_none());
    assert!(record.outcome.timer_ms.is_none());
}

#[test]
#[ignore = "explicit native headless keystone inputs; two 200 s runs of one seed"]
fn same_seed_headless_keystones_produce_identical_records() {
    const TICKS: usize = 4_000;
    let options = Options::from_environment().expect("explicit keystone runner inputs");
    let seed: u64 = std::env::var("KEYSTONE_SIMULATOR_SEED")
        .expect("keystone seed")
        .parse()
        .expect("numeric keystone seed");
    let run = || {
        serde_json::to_value(super::super::campaign::run_keystone_ticks(
            seed, options, TICKS,
        ))
        .expect("serialize keystone record")
    };
    let first = run();
    assert_eq!(first["schedule_epoch"], schedule_epoch().unwrap());
    let second = run();
    assert_eq!(
        first["affixes"], second["affixes"],
        "same inputs changed affixes"
    );
    assert!(
        first["outcome"]["forces"]
            .as_u64()
            .is_some_and(|forces| forces > 0),
        "the bounded run must reach combat: {}",
        first["outcome"]
    );
    assert!(
        first == second,
        "same seed and inputs produced different records"
    );
}

#[test]
#[ignore = "explicit full native headless keystone run; use scripts/run-keystone-simulator.py"]
fn run_headless_keystone() {
    let options = Options::from_environment().expect("explicit keystone runner inputs");
    let seed: u64 = std::env::var("KEYSTONE_SIMULATOR_SEED")
        .expect("keystone seed")
        .parse()
        .expect("numeric keystone seed");
    let output = std::path::PathBuf::from(
        std::env::var_os("KEYSTONE_SIMULATOR_OUTPUT").expect("keystone record path"),
    );
    let record = super::super::campaign::run_keystone(seed, options);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).expect("create record directory");
    }
    std::fs::write(
        &output,
        serde_json::to_vec_pretty(&record).expect("serialize keystone record"),
    )
    .expect("write keystone record");
    println!(
        "KEYSTONE status={} map={} key={} record={} reason={}",
        record.outcome.status,
        record.map_id,
        record.key_level,
        output.display(),
        record.outcome.reason
    );
    assert_eq!(
        record.outcome.status, "completed",
        "native keystone did not complete; inspect the record"
    );
}
