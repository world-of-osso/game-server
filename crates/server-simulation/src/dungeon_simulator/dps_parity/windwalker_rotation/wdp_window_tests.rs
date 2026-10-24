//! First real native enabler window; diagnostics never waive a cast/source gate.
use super::*;
use server::simc_apl::Program;

const WDP: u32 = 152175;
const ENABLER: u32 = 196742;
type Run = super::super::Run;

fn action() -> Action {
    Action {
        name: "whirling_dragon_punch".into(),
        target: None,
        options: Default::default(),
    }
}

fn read_enabler(run: &Run) -> Option<Value> {
    let world = run.app.world();
    let now = crate::spell_cast::now(world);
    world
        .get::<crate::auras::ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| {
            aura.spell_id == ENABLER
                && aura.caster == run.actor
                && aura.expires_at.is_none_or(|at| at > now)
        })
        .map(|aura| {
            json!({"instance": aura.instance_id, "duration_ms": aura.duration_ms,
            "expires_at": aura.expires_at, "now": now})
        })
}

fn read_source_conditions(facts: &WwFacts<'_>) -> Vec<Value> {
    include_str!("76f8118.simc")
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let (head, literal) = line.split_once('=')?;
            let literal = literal.trim_start_matches('/');
            if !literal.starts_with("whirling_dragon_punch") {
                return None;
            }
            let program = Program::parse(&[("row", literal)]).unwrap();
            let selected = program.select("row", facts, &mut program.state(), |_| Ok(true));
            Some(
                json!({"line": index + 1, "list": head.trim_end_matches('+'), "source": literal,
            "condition_selects": selected.map(|selected| selected.is_some())}),
            )
        })
        .collect()
}

fn read_native_admission(run: &mut Run, target: Entity) -> String {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(WDP)
        .cloned()
        .unwrap();
    match crate::spell_cast::read_native_cast_start(run.app.world_mut(), run.actor, &spell, target)
    {
        Ok(actual) => format!("ready:{actual:?}"),
        Err(reason) => format!("{reason:?}"),
    }
}

fn read_window_gates(run: &mut Run, rotation: &Rotation, prepared: &Prepared) -> Value {
    let xuen = facts::read_xuen(run.app.world_mut(), run.actor);
    let facts = WwFacts::new(
        run.app.world(),
        run.actor,
        &rotation.bindings,
        &rotation.equipment,
        rotation.encounter,
        &run.targets,
        xuen,
    );
    let source = read_source_conditions(&facts);
    let values: serde_json::Map<String, Value> = [
        "cooldown.whirling_dragon_punch.remains",
        "cooldown.rising_sun_kick.remains",
        "cooldown.fists_of_fury.remains",
        "cooldown.zenith.full_recharge_time",
        "buff.whirling_dragon_punch.up",
        "buff.whirling_dragon_punch.remains",
        "gcd.remains",
        "gcd.max",
        "chi",
        "energy",
        "talent.flurry_strikes",
    ]
    .into_iter()
    .map(|name| (name.into(), json!(facts.value_for(&action(), name))))
    .collect();
    let world = run.app.world();
    let known = world
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap()
        .contains(WDP);
    let resolved = literal_rotation::execution::resolve_action(world, run.actor, WDP);
    let cast = format!("{:?}", world.get::<shared::casting::CastState>(run.actor));
    let moving = world
        .get::<crate::networking::Moving>(run.actor)
        .map(|moving| moving.0);
    let any_gcd = world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .is_some_and(|history| history.has_active_gcd(crate::spell_cast::history_now(world)));
    let ready = prepared
        .execution(&action(), &rotation.equipment)
        .map(|ready| ready.is_some());
    let row_gate = prepared.permits_row(&action());
    let target = run.targets[0];
    let actor = run.actor;
    let enemy = read_native_admission(run, target);
    let self_cast = read_native_admission(run, actor);
    json!({"source_rows": source, "facts": values, "known": known, "resolved_action": resolved,
        "cast": cast, "moving": moving, "any_native_gcd": any_gcd,
        "prepared_ready": ready, "row_gate": row_gate,
        "native_enemy_admission": enemy, "native_self_admission": self_cast})
}

fn advance_literal_frame(run: &mut Run, rotation: &mut Rotation, capture: bool) -> Value {
    // Exact production drive ordering, one real source selection/dispatch per frame.
    rotation
        .cancel_channel(run.app.world_mut(), run.actor, &run.targets)
        .unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &rotation.equipment,
        bindings::execution_actions(),
        false,
    )
    .unwrap();
    let gates = capture.then(|| read_window_gates(run, rotation, &prepared));
    let selected = rotation
        .select(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            "default",
            &prepared,
        )
        .unwrap();
    let chosen = selected.as_ref().map(|(action, _)| {
        json!({
        "name": action.name, "target": action.target, "options": action.options})
    });
    if let Some((action, execution)) = selected {
        rotation
            .driver
            .execute(run.app.world_mut(), run.actor, action, execution)
            .unwrap();
    }
    run.app.update();
    json!({"gates": gates, "chosen": chosen})
}

#[test]
#[ignore = "native first WDP enabler window: exact source conditions and native refusal timeline"]
fn literal_ww_native_wdp_first_enabler_window_exposes_source_and_native_rejections() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    let start = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, start);
    super::super::engage(&mut run);
    let mut rotation = run.windwalker_rotation.take().unwrap();
    let mut seen = false;
    let mut closed_frames = 0;
    let mut trace = Vec::new();
    for frame in 0..600 {
        let aura = read_enabler(&run);
        let capture = seen || aura.is_some();
        seen |= aura.is_some();
        if seen && aura.is_none() {
            closed_frames += 1;
        }
        let now = crate::spell_cast::now(run.app.world());
        let result = advance_literal_frame(&mut run, &mut rotation, capture);
        if capture {
            trace.push(json!({"frame": frame, "time": now, "aura": aura, "result": result}));
        }
        if closed_frames >= 3 {
            break;
        }
    }
    assert!(
        seen,
        "actual native producer must open the first enabler window"
    );
    let output = super::super::inputs::directory()
        .unwrap()
        .join("native-wdp-first-window.json");
    std::fs::write(&output, serde_json::to_vec_pretty(&trace).unwrap()).unwrap();
    eprintln!("NATIVE_WDP_FIRST_WINDOW {}", output.display());
}
