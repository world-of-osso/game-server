//! Observe complete native Barbed lifetimes before diagnosing residual damage loss.
//! No assigned stats/resources/timers, forced crits or inferred damage multipliers.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const BARBED_SHOT: u32 = 217200;
const MAX_DIAGNOSTIC_FRAMES: usize = 2400;
const MAX_CAPTURED_HITS: usize = 512;

fn prepare_budget_player() -> Run {
    let (profile, traits) = read_reference(253).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(BARBED_SHOT)
    );
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        BARBED_SHOT,
        "barbed_shot",
        MAX_CAPTURED_HITS,
    ));
    run
}

fn aura_snapshot(run: &Run) -> Option<Value> {
    let world = run.app.world();
    let aura = world
        .get::<ActiveAuras>(run.targets[0])?
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == BARBED_SHOT)?;
    Some(
        json!({"time": crate::spell_cast::now(world), "instance": aura.instance_id,
        "duration_ms": aura.duration_ms, "expires_at": aura.expires_at, "stacks": aura.stacks,
        "effects": aura.effects.iter().map(|effect| json!({"index": effect.index,
            "amount": effect.amount, "period_ms": effect.period_ms,
            "aura_type": effect.aura_type})).collect::<Vec<_>>()}),
    )
}

fn barbed_gcd_ready(run: &Run) -> bool {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(BARBED_SHOT)
        .unwrap();
    !world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .is_some_and(|history| history.on_gcd(spell, crate::spell_cast::history_now(world)))
}

fn cast_budget_barbed(run: &mut Run, casts: &mut Vec<Value>) {
    let actor = run.actor;
    let target = run.targets[0];
    let at = crate::spell_cast::now(run.app.world());
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, BARBED_SHOT, Some(target), false)
        .expect("real known Player Barbed must pass ordinary admission");
    casts.push(json!({"at": at, "actor": actor.to_bits(), "target": target.to_bits()}));
}

fn observe_lifetime(run: &mut Run, reapply_when_ready: bool) -> (Vec<Value>, Vec<Value>) {
    let mut casts = Vec::new();
    let mut timeline = Vec::new();
    let mut seen = false;
    cast_budget_barbed(run, &mut casts);
    for _ in 0..MAX_DIAGNOSTIC_FRAMES {
        run.app.update();
        if let Some(snapshot) = aura_snapshot(run) {
            seen = true;
            timeline.push(snapshot);
            if reapply_when_ready && casts.len() == 1 && barbed_gcd_ready(run) {
                cast_budget_barbed(run, &mut casts);
            }
        } else if seen && (!reapply_when_ready || casts.len() == 2) {
            return (casts, timeline);
        }
    }
    panic!("native Barbed did not finish its actual lifetime within diagnostic bound");
}

fn write_budget_receipt(
    run: &Run,
    name: &str,
    casts: Vec<Value>,
    timeline: Vec<Value>,
    before: Value,
) {
    let world = run.app.world();
    let mut capture = world.resource::<FactorCapture>().export();
    let events = &world.resource::<ObservedCombatEvents>().0;
    let successes = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(BARBED_SHOT)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    assert_eq!(
        successes,
        casts.len(),
        "no hidden rotation or extra Barbed casts"
    );
    let tick_inputs: Vec<_> = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "result" && event["spell_id"] == BARBED_SHOT)
        .map(|event| {
            event["factors"]["native_input_amount"]
                .as_f64()
                .expect("existing factor capture must expose actual periodic source input")
        })
        .collect();
    assert!(!tick_inputs.is_empty() && tick_inputs.iter().all(|amount| *amount > 0.0));
    assert!(
        aura_snapshot(run).is_none(),
        "capture includes final removal, not a pending tail"
    );
    let expiry = timeline.last().unwrap()["expires_at"].as_f64().unwrap();
    let now = crate::spell_cast::now(world);
    assert!(
        now + 1e-6 >= expiry,
        "native aura must reach its actual expiry, not an early clear"
    );
    let elapsed = now - casts.first().unwrap()["at"].as_f64().unwrap();
    capture["elapsed_seconds"] = json!(elapsed);
    capture["casts"] = json!(casts);
    capture["aura_timeline"] = json!(timeline);
    capture["stats_before"] = before;
    capture["stats_after"] = report::stats(world, run.actor);
    capture["raw_tick_input_sum"] = json!(tick_inputs.iter().sum::<f64>());
    capture["raw_tick_inputs"] = json!(tick_inputs);
    capture["combat"] = report::combat(world, &run.damage_targets, elapsed);
    capture["qualification"] = json!(
        "native complete-lifetime diagnostic, not gameplay RED or whole-fight DPS certification"
    );
    let root =
        std::path::PathBuf::from(std::env::var_os("DPS_BARBED_OUTPUT").expect("DPS_BARBED_OUTPUT"));
    let output = root.join(format!("{name}.json"));
    assert!(
        !output.exists(),
        "do not overwrite a native diagnostic receipt"
    );
    std::fs::write(output, serde_json::to_vec_pretty(&capture).unwrap()).unwrap();
}

#[test]
#[ignore = "native Barbed lifetime factors; requires prepared253/current world and DPS_BARBED_OUTPUT"]
fn dps_parity_barbed_budget_native_isolated_cast_records_complete_lifetime() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_budget_player();
    let before = report::stats(run.app.world(), run.actor);
    let (casts, timeline) = observe_lifetime(&mut run, false);
    assert_eq!(casts.len(), 1);
    write_budget_receipt(&run, "isolated", casts, timeline, before);
}

#[test]
#[ignore = "native Barbed reapplication factors; requires prepared253/current world and DPS_BARBED_OUTPUT"]
fn dps_parity_barbed_budget_native_legal_reapplication_records_complete_lifetime() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_budget_player();
    let before = report::stats(run.app.world(), run.actor);
    let (casts, timeline) = observe_lifetime(&mut run, true);
    assert_eq!(casts.len(), 2);
    write_budget_receipt(&run, "reapplied", casts, timeline, before);
}
