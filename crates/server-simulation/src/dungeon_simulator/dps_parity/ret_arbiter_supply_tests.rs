//! Passive native lifecycle counts separate Arbiter supply from damage/seed variance.
use super::*;
use crate::auras::observations;

const TRACKED: &[(u32, u32)] = &[
    (408458, 408459),
    (1306161, 1296661),
    (1306162, 1296661),
    (1310461, 1296661),
    (387178, 387170),
];
const ARBITER_KINDS: &[u32] = &[1306161, 1306162, 1310461];

#[test]
#[ignore = "native three-seed Ret300s Purpose/Arbiter supply observation"]
fn ret_arbiter_supply_records_native_initial_uses_applications_consumption_and_payouts() {
    for seed in [70448, 1, 2] {
        let _seed = crate::random::seed_scope(seed);
        let mut run = prepare_measurement(70, 1).expect("native Ret follower measurement");
        observations::start_tracking(run.app.world_mut(), run.actor, TRACKED);
        let initial = observations::snapshot(run.app.world());
        for spell in [408458, 1306161, 1306162, 1310461] {
            assert_eq!(
                initial[spell.to_string()]["initial_charges"],
                0,
                "native setup cannot start with unearned Purpose/Arbiter uses"
            );
        }
        engage(&mut run);
        let (seconds, uptimes, _) = advance_combat(&mut run).expect("ordinary native300s combat");
        let combat = report::combat(run.app.world(), &run.damage_targets, seconds);
        let usage = observations::snapshot(run.app.world());
        let payouts: u64 = ARBITER_KINDS
            .iter()
            .map(|id| usage[id.to_string()]["consumed"].as_u64().unwrap())
            .sum();
        let arbiter = combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|action| action["spell_id"] == 1306923)
            .expect("native Arbiter damage");
        let in_flight = crate::spell_cast::native_apl::spell_hits_in_flight(
            run.app.world(),
            run.actor,
            1306923,
        )
        .unwrap() as u64;
        let outcomes: u64 = ["hits", "misses", "dodges", "parries"]
            .iter()
            .map(|key| arbiter[*key].as_u64().unwrap())
            .sum();
        eprintln!(
            "NATIVE_ARBITER_WINDOW seed={seed} consumed={payouts} outcomes={outcomes} in_flight={in_flight}"
        );
        assert_eq!(
            payouts,
            outcomes + in_flight,
            "every consumed native Arbiter use must have a real outcome or remain in actual flight at300s"
        );
        let output = json!({"seed":seed,"simulated_seconds":seconds,"initial_uses":initial,
            "usage":usage,"arbiter_payouts":payouts,"arbiter":arbiter,
            "arbiter_terminal_outcomes":outcomes,"arbiter_in_flight_at_end":in_flight,
            "buff_uptime_pct":uptimes,"combat":combat,"stats_after":report::stats(run.app.world(),run.actor)});
        let path = inputs::directory()
            .unwrap()
            .join(format!("70.arbiter_supply.seed-{seed}.native.json"));
        std::fs::write(&path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
        eprintln!(
            "NATIVE_ARBITER_SUPPLY seed={seed} payouts={payouts} usage={} path={}",
            output["usage"],
            path.display()
        );
    }
}
