//! Bounded native spell probes using the shared literal fixture/dispatch path.
use super::*;
use crate::simc_apl_native::{Encounter, FightStyle};
use literal_rotation::capture_fixture;
use literal_rotation::equipment::Equipment;
use literal_rotation::execution::{NativeAction, Prepared};
use server::simc_apl::Action;
use shared::protocol::CombatLogKind;

const MAX_FRAMES: usize = 2_400;
const FRAME_SECONDS: f64 = 0.05;

struct ProbeSnapshot {
    stats: Value,
    targets: Value,
    auras: Value,
    started_at: f64,
}

pub(crate) struct SpellProbe<'a> {
    pub spec: u32,
    pub spell: u32,
    pub damage_spell: u32,
    pub action: &'a str,
    pub ticks: usize,
}

fn capture_controller(run: &mut Run) -> Result<Equipment, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: MAX_FRAMES as f64 * FRAME_SECONDS,
        style: FightStyle::Patchwerk,
    };
    capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    engage_fixture_opponents(run);
    Equipment::load(run.app.world(), run.actor, &mut Default::default())
}

fn read_probe_execution(
    run: &mut Run,
    probe: &SpellProbe,
    equipment: &Equipment,
) -> Result<Option<literal_rotation::execution::Execution>, String> {
    let actions = [NativeAction {
        name: probe.action,
        spell: probe.spell,
        talent: None,
        self_target: false,
    }];
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        equipment,
        actions,
        false,
    )?;
    let action = Action {
        name: probe.action.into(),
        options: Default::default(),
    };
    prepared.execution(&action, equipment)
}

fn prime_auto_attack(run: &mut Run) -> Result<(), String> {
    if run
        .app
        .world()
        .get::<crate::combat::AutoAttack>(run.actor)
        .is_none()
    {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
        )?;
    }
    Ok(())
}

fn dispatch_once(run: &mut Run, probe: &SpellProbe) -> Result<(), String> {
    let equipment = capture_controller(run)?;
    for _ in 0..MAX_FRAMES {
        if let Some(execution) = read_probe_execution(run, probe, &equipment)? {
            literal_rotation::execution::execute(run.app.world_mut(), run.actor, execution)?;
            run.app
                .world_mut()
                .entity_mut(run.actor)
                .remove::<crate::combat::AutoAttack>();
            return Ok(());
        }
        prime_auto_attack(run)?;
        run.app.update();
    }
    Err(format!(
        "native spell{} never reached literal readiness",
        probe.spell
    ))
}

fn observed_hits(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
        })
        .count()
}

pub(crate) fn run_spell_probe(probe: SpellProbe) -> Result<Value, String> {
    if probe.ticks == 0 {
        return Err("a spell probe requires at least one hit/tick".into());
    }
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(probe.spec, 1)?;
    let snapshot = begin_capture(&mut run, &probe)?;
    dispatch_once(&mut run, &probe)?;
    for _ in 0..MAX_FRAMES {
        run.app.update();
        if observed_hits(&run, probe.damage_spell) >= probe.ticks {
            return export_receipt(&run, &probe, snapshot);
        }
    }
    Err(format!(
        "native spell{} did not produce {} hits/ticks",
        probe.spell, probe.ticks
    ))
}

fn begin_capture(run: &mut Run, probe: &SpellProbe) -> Result<ProbeSnapshot, String> {
    verify_raid_free(run.app.world(), run.actor)?;
    let snapshot = ProbeSnapshot {
        stats: report::stats(run.app.world(), run.actor),
        targets: json!(target_snapshot(run)),
        auras: aura_snapshot(run.app.world(), run.actor),
        started_at: crate::spell_cast::now(run.app.world()),
    };
    run.app.world_mut().insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            run.actor,
            run.targets[0],
            probe.damage_spell,
            probe.action,
            probe.ticks,
        ),
    );
    Ok(snapshot)
}

fn verify_raid_free(world: &World, actor: Entity) -> Result<(), String> {
    for spell in [6673, 1459, 1126, 21562] {
        if crate::auras::has_aura(world, actor, spell) {
            return Err(format!(
                "external raid buff{spell} present on native probe actor"
            ));
        }
    }
    Ok(())
}

fn aura_snapshot(world: &World, actor: Entity) -> Value {
    let auras = world
        .get::<crate::auras::ActiveAuras>(actor)
        .map(|auras| {
            auras
                .iter()
                .map(|aura| {
                    json!({"spell_id": aura.spell_id, "caster": aura.caster.to_bits(),
            "source": format!("{:?}", aura.source), "stacks": aura.stacks,
            "effects": aura.effects.iter().map(|effect| json!({"index": effect.index,
                "aura_type": effect.aura_type, "amount": effect.amount})).collect::<Vec<_>>()})
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!(auras)
}

fn observed_casts(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count()
}

fn append_reference_context(run: &Run, receipt: &mut Value) -> Result<(), String> {
    receipt["gaps"] = json!(run.gaps);
    receipt["reference_directory"] = json!(inputs::directory()?);
    receipt["native_database"] =
        json!(std::env::var_os("DPS_PARITY_WORLD_DB").map(std::path::PathBuf::from));
    receipt["reference_seed"] = json!(70448);
    receipt["raid_buff_check"] = json!({"absent": [6673, 1459, 1126, 21562], "fixture": "standalone actor; no external raid buffers"});
    Ok(())
}

fn export_receipt(run: &Run, probe: &SpellProbe, snapshot: ProbeSnapshot) -> Result<Value, String> {
    let world = run.app.world();
    verify_raid_free(world, run.actor)?;
    let mut receipt = world
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    receipt["spec_id"] = json!(probe.spec);
    receipt["cast_spell_id"] = json!(probe.spell);
    receipt["damage_spell_id"] = json!(probe.damage_spell);
    receipt["casts"] = json!(observed_casts(run, probe.spell));
    receipt["stats_before"] = snapshot.stats;
    receipt["stats_after"] = report::stats(world, run.actor);
    receipt["targets_before"] = snapshot.targets;
    receipt["targets_after"] = json!(target_snapshot(run));
    receipt["aura_state_before"] = snapshot.auras;
    receipt["aura_state_after"] = aura_snapshot(world, run.actor);
    receipt["combat"] = report::combat(
        world,
        &run.targets,
        crate::spell_cast::now(world) - snapshot.started_at,
    );
    append_reference_context(run, &mut receipt)?;
    Ok(receipt)
}

#[test]
#[ignore = "real reference gear/traits and31146; frozen source DB via dps_parity_native.py"]
fn dps_parity_single_spell_tiger_palm_records_real_factors() {
    let receipt = run_spell_probe(SpellProbe {
        spec: 269,
        spell: 100780,
        damage_spell: 100780,
        action: "tiger_palm",
        ticks: 1,
    })
    .expect("actual native Tiger Palm");
    let output = std::env::var_os("DPS_SPELL_OUTPUT").expect("DPS_SPELL_OUTPUT");
    std::fs::write(output, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    let events = receipt["events"]
        .as_array()
        .expect("per-hit formula receipts, not aggregate damage");
    let formula = events
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 100780)
        .expect("actual Tiger Palm formula");
    assert!(formula["factors"]["a_power"].as_f64().unwrap() > 0.0);
    assert!(formula["factors"]["a_mod"].as_f64().unwrap() > 0.0);
    assert!(formula["factors"]["base"].is_number());
    assert!(events.iter().any(|event| {
        event["event"] == "result"
            && event["factors"]["actual_amount"]
                .as_f64()
                .is_some_and(|damage| damage > 0.0)
    }));
    assert_eq!(receipt["casts"], 1);
}
