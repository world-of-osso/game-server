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
    let owner = world
        .spawn((
            shared::components::Health {
                current: 75.0,
                max: 100.0,
            },
            shared::components::Mana {
                current: 20.0,
                max: 100.0,
            },
        ))
        .id();
    let party = [owner];
    let tank = TankProgress {
        position: Some([1.0, 2.0, 3.0]),
        state: Some(&idle),
        follow: Some(&follow),
    };
    let observation = |current_pack, decisions| Observation {
        current_pack,
        decisions,
        party: &party,
        world: &world,
        owner,
        tank,
    };
    reporter
        .write(&mut output, 0, &snapshot, &route, observation(None, &[]))
        .unwrap();
    assert!(output.is_empty(), "countdown is not running time");
    snapshot.phase = ChallengePhase::Running;
    reporter
        .write(&mut output, 0, &snapshot, &route, observation(None, &[]))
        .unwrap();
    let decisions = [super::super::campaign::PullDecision {
        at: 10.0,
        pack: 10,
        decision: "Wait(Recovery)".into(),
    }];
    reporter
        .write(
            &mut output,
            10_000,
            &snapshot,
            &route,
            observation(Some(10), &decisions),
        )
        .unwrap();
    reporter
        .write(
            &mut output,
            59_999,
            &snapshot,
            &route,
            observation(Some(10), &decisions),
        )
        .unwrap();
    snapshot.forces = 75;
    snapshot.deaths = 1;
    snapshot.bosses[0].defeated = true;
    reporter
        .write(
            &mut output,
            60_000,
            &snapshot,
            &route,
            observation(Some(8_100_243_029), &decisions),
        )
        .unwrap();
    reporter
        .write(
            &mut output,
            60_050,
            &snapshot,
            &route,
            observation(Some(8_100_243_029), &decisions),
        )
        .unwrap();
    let decisions = [
        super::super::campaign::PullDecision {
            at: 10.0,
            pack: 10,
            decision: "Wait(Recovery)".into(),
        },
        super::super::campaign::PullDecision {
            at: 60.1,
            pack: 10,
            decision: "Allow".into(),
        },
        super::super::campaign::PullDecision {
            at: 60.1,
            pack: 11,
            decision: "Wait(Healer)".into(),
        },
    ];
    reporter
        .write(
            &mut output,
            60_100,
            &snapshot,
            &route,
            observation(Some(11), &decisions),
        )
        .unwrap();
    let output = String::from_utf8(output).unwrap();
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 5, "{output}");
    assert!(lines[0].contains("sim_ms=0 forces=0/655 bosses=0/2"));
    assert!(lines[1].contains("sim_ms=10000"));
    assert!(
        lines[1]
            .contains("route_step=Some(1)/2 decision_pack=Some(10) decision=\"Wait(Recovery)\"")
    );
    assert!(lines[1].contains(&format!("party={}:75.0:20.0:alive", owner.to_bits())));
    assert!(lines[1].contains("enemies_in_combat=0"));
    assert!(lines[2].contains("sim_ms=60000 forces=75/655 bosses=1/2"));
    assert!(lines[3].contains("decision_pack=Some(10) decision=\"Allow\""));
    assert!(lines[4].contains("decision_pack=Some(11) decision=\"Wait(Healer)\""));
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
#[ignore = "explicit native headless keystone inputs; two 200 s runs of one seed"]
fn same_seed_and_epoch_select_identical_affixes() {
    const TICKS: usize = 4_000;
    let options = Options::from_environment().expect("explicit keystone runner inputs");
    let seed: u64 = std::env::var("KEYSTONE_SIMULATOR_SEED")
        .expect("keystone seed")
        .parse()
        .expect("numeric keystone seed");
    let run = || super::super::campaign::run_keystone_ticks(seed, options, TICKS);
    let first = run();
    let second = run();
    assert_eq!(first.schedule_epoch, Some(schedule_epoch().unwrap()));
    assert_eq!(first.schedule_epoch, second.schedule_epoch);
    assert!(first.affixes.is_some(), "run must activate the keystone");
    assert_eq!(first.affixes, second.affixes);
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
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).expect("create record directory");
    }
    let mut journal_path = output.as_os_str().to_os_string();
    journal_path.push(".pulls.jsonl");
    let mut journal = std::fs::File::create(&journal_path).expect("create pull decision journal");
    let record = super::super::campaign::run_keystone_with_journal(seed, options, &mut journal);
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
    if let Err(failure) = super::super::fact_error_report::check_fact_errors(&record.fact_errors) {
        panic!("native keystone recorded {failure}");
    }
    assert_eq!(
        record.outcome.status, "completed",
        "native keystone did not complete; inspect the record"
    );
}
