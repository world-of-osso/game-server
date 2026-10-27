//! Passive native lifecycle counts separate Arbiter supply from damage/seed variance.
use super::*;
use crate::auras::observations;
use crate::spell_cast::test_harness::{
    ObservedCombatEvents, ObservedDiscardedSpellHits, ObservedTriggeredCasts,
};
use shared::protocol::CombatLogKind;

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
        run.app.init_resource::<ObservedTriggeredCasts>();
        run.app.init_resource::<ObservedDiscardedSpellHits>();
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
        let started_at = run.app.world().resource::<Time>().elapsed();
        let mut pending_before_teardown = 0;
        let mut teardown_start_native = 0.0;
        let (seconds, uptimes, _) = advance_combat_observed(&mut run, |run| {
            let world = run.app.world();
            if world
                .get::<shared::components::Health>(run.targets[0])
                .unwrap()
                .current
                > 0.0
            {
                teardown_start_native = world.resource::<Time<Fixed>>().elapsed_secs_f64();
                pending_before_teardown =
                    crate::spell_cast::native_apl::spell_hits_in_flight(world, run.actor, 1306923)
                        .unwrap() as u64;
            } else {
                assert!(
                    world.resource::<Time>().elapsed() >= started_at + FIGHT_DURATION,
                    "fixed-time primary cannot die or be replaced mid-fight"
                );
            }
        })
        .expect("ordinary native300s combat");
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
        let launches = run
            .app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .iter()
            .filter(|(caster, spell, _)| *caster == run.actor && *spell == 1306923)
            .count() as u64;
        let events: Vec<_> = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(1306923)
            })
            .collect();
        // The presentation report counts only Miss/Dodge/Parry, not every native miss kind.
        let outcomes = events
            .iter()
            .filter(|event| matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_)))
            .count() as u64;
        let misses: Vec<_> = events
            .iter()
            .filter(|event| matches!(event.kind, CombatLogKind::Miss(_)))
            .collect();
        let discarded: Vec<_> = run.app.world().resource::<ObservedDiscardedSpellHits>().0.iter()
            .filter(|hit| hit.caster == run.actor && hit.spell_id == 1306923)
            .map(|hit| json!({"caster":hit.caster.to_bits(),"target":hit.target.to_bits(),
                "launched_at":hit.launched_at,"due_at":hit.due_at,"discarded_at":hit.discarded_at,
                "alive_at_launch":hit.alive_at_launch,"exists_now":hit.exists_now,"alive_now":hit.alive_now}))
            .collect();
        eprintln!(
            "NATIVE_ARBITER_WINDOW seed={seed} target={} cutoff={} consumed={payouts} launches={launches} outcomes={outcomes} pending_before_teardown={pending_before_teardown} in_flight_after={in_flight} misses={misses:?} discarded={discarded:?}",
            run.targets[0].to_bits(),
            (started_at + FIGHT_DURATION).as_secs_f64()
        );
        let teardown_end_native = run.app.world().resource::<Time<Fixed>>().elapsed_secs_f64();
        for hit in run
            .app
            .world()
            .resource::<ObservedDiscardedSpellHits>()
            .0
            .iter()
            .filter(|hit| hit.caster == run.actor && hit.spell_id == 1306923)
        {
            assert!(
                teardown_start_native < hit.discarded_at && hit.discarded_at <= teardown_end_native,
                "Arbiter discard occurred outside the final target-teardown frame: {hit:?}"
            );
            assert!(
                hit.alive_at_launch && hit.exists_now && !hit.alive_now,
                "fixed-time target must exist and change alive state only at teardown: {hit:?}"
            );
        }
        assert_eq!(
            pending_before_teardown,
            in_flight + discarded.len() as u64,
            "pre-teardown native flights must remain queued or have an actual teardown discard witness"
        );
        assert_eq!(
            payouts, launches,
            "every consumed native use must launch its real payload"
        );
        assert_eq!(
            payouts,
            outcomes + pending_before_teardown,
            "every consumed native Arbiter use must have a real outcome or remain in actual flight at300s"
        );
        let output = json!({"seed":seed,"simulated_seconds":seconds,"initial_uses":initial,
            "usage":usage,"arbiter_payouts":payouts,"arbiter":arbiter,
            "arbiter_launches":launches,"arbiter_terminal_outcomes":outcomes,
            "arbiter_in_flight_at_end":pending_before_teardown,"arbiter_in_flight_after_teardown":in_flight,
            "arbiter_discarded_hits":discarded,
            "arbiter_native_misses":misses,
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
