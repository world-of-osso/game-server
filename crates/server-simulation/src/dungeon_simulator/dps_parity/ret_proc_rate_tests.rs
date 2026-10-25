//! Frozen SimC76f8118 MID2 proc rates compared with actual native300s attempts.
use super::*;
use crate::spell_cast::test_harness::ObservedTriggeredCasts;

const CRUSADING_STRIKE: u32 = 408385;
const ART_OF_WAR: u32 = 406086;
// Frozen70.reference.{single,three}.json: owner melee.num_executes.mean and
// owner procs["Art of War"].count. Counts average999 seeded reference fights.
const REFERENCE: &[(usize, f64, f64)] = &[
    (1, 192.2112112112112, 30.66966966966967),
    (3, 192.52052052052053, 30.855855855855857),
];

#[test]
#[ignore = "native70448 Ret300s proc count must match pinned SimC opportunity/rate"]
fn ret_art_of_war_native_proc_count_matches_reference_rate() {
    let mut failures = Vec::new();
    for &(targets, reference_attempts, reference_procs) in REFERENCE {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(70, targets).unwrap();
        run.app.init_resource::<ObservedTriggeredCasts>();
        engage(&mut run);
        let (_, uptimes, _) = advance_combat(&mut run).unwrap();
        let casts = &run.app.world().resource::<ObservedTriggeredCasts>().0;
        let count_casts = |spell| {
            casts
                .iter()
                .filter(|(caster, id, _)| *caster == run.actor && *id == spell)
                .count() as f64
        };
        let attempts = count_casts(CRUSADING_STRIKE);
        let procs = count_casts(ART_OF_WAR);
        let chance_effects = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 406064)
            .unwrap()
            .effects
            .iter()
            .map(|effect| (effect.index, effect.amount))
            .collect::<Vec<_>>();
        //192 opportunities are one fight, not999: a three-sigma binomial
        // window detects a real rate deficit without demanding an exact RNG path.
        let reference_rate = reference_procs / reference_attempts;
        let expected = attempts * reference_rate;
        let tolerance = 3.0 * (attempts * reference_rate * (1.0 - reference_rate)).sqrt();
        eprintln!(
            "AOW_NATIVE targets={targets} trigger={CRUSADING_STRIKE} attempts={attempts} procs={procs} expected={expected} tolerance={tolerance} live_chance_effects={chance_effects:?} uptime={:?}",
            uptimes.get(&406086)
        );
        if (attempts / reference_attempts - 1.0).abs() > 0.10 {
            failures.push(format!("targets{targets}: native repeating attack volume {attempts} differs from reference{reference_attempts}"));
        }
        if (procs - expected).abs() > tolerance {
            failures.push(format!("targets{targets}: native Art of War {procs}/{attempts} outside reference rate{reference_rate}; expected{expected}±{tolerance}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("; "));
}
