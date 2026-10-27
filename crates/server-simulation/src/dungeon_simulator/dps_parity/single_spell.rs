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
const PRIMER_SETTLE_FRAMES: usize = 10;

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
        source_row: None,
        name: probe.action.into(),
        options: Default::default(),
        target: None,
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
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(probe.spec, 1)?;
    capture_cast(&mut run, &probe)
}

/// Callers may prime their existing real fixture with shared native execution first.
pub(super) fn capture_cast(run: &mut Run, probe: &SpellProbe) -> Result<Value, String> {
    if probe.ticks == 0 || run.profile.spec_id != probe.spec {
        return Err(
            "native probe requires positive hit count and matching fixture specialization".into(),
        );
    }
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    let snapshot = begin_capture(run, probe)?;
    dispatch_once(run, probe)?;
    for _ in 0..MAX_FRAMES {
        run.app.update();
        if observed_hits(run, probe.damage_spell) >= probe.ticks {
            return export_receipt(run, probe, snapshot);
        }
    }
    Err(format!(
        "native cast{} did not produce {} hits for spell{}",
        probe.spell, probe.ticks, probe.damage_spell
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
    let damage_action = if probe.spell == probe.damage_spell {
        probe.action.to_string()
    } else {
        format!("spell_{}", probe.damage_spell)
    };
    run.app.world_mut().insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            run.actor,
            run.targets[0],
            probe.damage_spell,
            &damage_action,
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
    assert!(formula["native_emission_origin"].is_null());
    assert!(events.iter().any(|event| {
        event["event"] == "result"
            && event["factors"]["actual_amount"]
                .as_f64()
                .is_some_and(|damage| damage > 0.0)
    }));
    assert_eq!(receipt["casts"], 1);
}

#[test]
#[ignore = "actual Arms Player reference fixture; requires shared Player explicit-target dispatch"]
fn dps_parity_player_single_spell_mortal_strike_records_real_factors() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(71, 1).expect("actual Arms reference; use DPS_PARITY_ACTOR=player");
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some(),
        "Arms uses a real Player, not an invented follower profile"
    );
    let receipt = capture_cast(
        &mut run,
        &SpellProbe {
            spec: 71,
            spell: 12294,
            damage_spell: 12294,
            action: "mortal_strike",
            ticks: 1,
        },
    )
    .expect("actual shared-dispatch Mortal Strike");
    eprintln!(
        "native Arms2pc1296643: eligible={:?}, aura_present={}, equipped={:?}",
        crate::item_proc_data::has_equipped_bonus(run.app.world(), run.actor, 1296643),
        crate::auras::has_aura(run.app.world(), run.actor, 1296643),
        run.app
            .world()
            .get::<crate::stats::EquippedItems>(run.actor)
            .unwrap()
            .items
    );
    let output =
        std::path::PathBuf::from(std::env::var_os("DPS_SPELL_OUTPUT").expect("DPS_SPELL_OUTPUT"))
            .with_file_name("71.mortal_strike.player.json");
    std::fs::write(output, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert_eq!(receipt["casts"], 1);
    assert_eq!(receipt["captured_hits"], 1);
    let formula = receipt["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 12294)
        .expect("actual Mortal Strike formula");
    assert!(formula["factors"]["a_power"].as_f64().unwrap() > 0.0);
    assert!(formula["factors"]["a_mod"].as_f64().unwrap() > 0.0);
}

#[test]
#[ignore = "real WW follower/reference280 policy; native armor-only receipt"]
fn dps_parity_single_spell_follower_armor_penetration() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(269, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some(),
        "this policy test requires DPS_PARITY_ACTOR=follower"
    );
    let receipt = capture_cast(
        &mut run,
        &SpellProbe {
            spec: 269,
            spell: 100780,
            damage_spell: 100780,
            action: "tiger_palm",
            ticks: 1,
        },
    )
    .expect("actual native follower Tiger Palm");
    let output =
        std::path::PathBuf::from(std::env::var_os("DPS_SPELL_OUTPUT").expect("DPS_SPELL_OUTPUT"))
            .with_file_name("269.tiger_palm.follower_armor.json");
    std::fs::write(output, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    let armor = receipt["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "armor" && event["spell_id"] == 100780)
        .expect("actual armor stage")["factors"]["armor_fraction"]
        .as_f64()
        .unwrap();
    // ExpectedStat475/478: armor1470/K3430; native450990/280:12%.
    // Capture the actual formula variable; rounded damage-stage ratios stay separate.
    let expected = (1470.0 * 0.88) / (1470.0 * 0.88 + 3430.0);
    assert!(
        (armor - expected).abs() < 0.000001,
        "FOLLOWER_CLASS_ACTOR_PLAYER_EQUIVALENT_ARMOR_PENETRATION: {armor} vs {expected}"
    );
    let result = receipt["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == 100780)
        .unwrap();
    let effective_armor = result["factors"]["target_armor"].as_f64().unwrap();
    assert!(
        (effective_armor - 1293.6).abs() < 0.001,
        "capture must expose actual post280 armor, not its pre-penetration input: {effective_armor}"
    );
    let armor_event = receipt["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "armor" && event["spell_id"] == 100780)
        .unwrap();
    assert_eq!(armor_event["factors"]["armor_coefficient"], 3430.0);
}

#[test]
#[ignore = "real reference gear/traits; actual native TP priming and three FoF child ticks"]
fn dps_parity_single_spell_fists_of_fury_three_ticks_keep_child_ids() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(269, 1).unwrap();
    let palm = SpellProbe {
        spec: 269,
        spell: 100780,
        damage_spell: 100780,
        action: "tiger_palm",
        ticks: 1,
    };
    for _ in 0..2 {
        dispatch_once(&mut run, &palm).expect("actual native TP earns Chi");
        for _ in 0..PRIMER_SETTLE_FRAMES {
            run.app.update();
        }
    }
    let probe = SpellProbe {
        spec: 269,
        spell: 113656,
        damage_spell: 117418,
        action: "fists_of_fury",
        // The native MID2 two-piece emits a separate half-strength child at execution.
        // Capture that child plus three actual scheduled/initial periodic emissions.
        ticks: 4,
    };
    let mut receipt = capture_cast(&mut run, &probe)
        .expect("one native tier child plus three real FoF periodic hits");
    receipt["priming"] = json!({"spells": [100780, 100780], "policy": "actual shared native execution; no granted Chi or buff stacks"});
    let output =
        std::path::PathBuf::from(std::env::var_os("DPS_SPELL_OUTPUT").expect("DPS_SPELL_OUTPUT"))
            .with_file_name("269.fists_of_fury.three_ticks.json");
    std::fs::write(output, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert_eq!(receipt["casts"], 1);
    assert_eq!(receipt["captured_hits"], 4);
    let all_amounts = receipt["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "direct_amount" && event["spell_id"] == 117418)
        .collect::<Vec<_>>();
    let amounts = all_amounts
        .iter()
        .copied()
        .filter(|event| event["native_emission_origin"]["kind"] == "periodic_dummy_trigger")
        .collect::<Vec<_>>();
    assert_eq!(amounts.len(), 3);
    let tier = all_amounts
        .iter()
        .copied()
        .filter(|event| event["native_emission_origin"].is_null())
        .collect::<Vec<_>>();
    assert_eq!(
        tier.len(),
        1,
        "the equipped MID2 child is independent of periodic emissions"
    );
    let tier_scale = tier[0]["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["factor"] == "cast_effectiveness")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    assert!(
        (tier_scale - 0.5).abs() < 0.00001,
        "native MID2 execution child retains half effectiveness"
    );
    for amount in &amounts {
        let origin = &amount["native_emission_origin"];
        assert_eq!(origin["kind"], "periodic_dummy_trigger");
        assert_eq!(origin["aura_spell_id"], 113656);
        assert_eq!(origin["effect_index"], 2);
        assert_eq!(origin["triggered_spell_id"], 117418);
    }
    let times = amounts
        .iter()
        .map(|event| event["time"].as_f64().unwrap())
        .collect::<Vec<_>>();
    assert!(
        times.windows(2).all(|pair| pair[0] < pair[1]),
        "three actual tick times, not duplicate result stages"
    );
    assert!(amounts.iter().all(|event| {
        event["factors"]["a_power"]
            .as_f64()
            .is_some_and(|power| power > 0.0)
    }));
}
