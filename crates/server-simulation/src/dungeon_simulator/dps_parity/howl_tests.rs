//! SIMC_BM_HOWL_OF_THE_PACK_LEADER_CYCLE: actual native actors, never fake readiness.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

const HOWL: u32 = 471876;
const HOWL_COOLDOWN: u32 = 471877;
const WYVERN_READY: u32 = 471878;
const WYVERN_SUMMON: u32 = 1222271;
const WYVERN_CRY: u32 = 471881;
const WYVERN_ENTRY: u32 = 234170;
const BOAR_IMPACT: u32 = 471936;
const BEAR_SUMMON: u32 = 471990;
const KILL_COMMAND: u32 = 34026;
const NEARBY_FIXTURE_TARGET_OFFSET: f32 = 1.5;

fn prepare_howl_player() -> Run {
    let (profile, traits) = read_reference(253).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    load_native_collision(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, HOWL));
    let data = run.app.world().resource::<crate::world_data::WorldData>();
    assert!(
        data.templates.contains_key(&WYVERN_ENTRY),
        "normal authentic summon closure is a prerequisite, not gameplay RED"
    );
    run
}

fn load_native_collision(run: &mut Run) {
    crate::test_data::install_native_collision(&mut run.app)
        .expect("actual source ground and LOS, not synthetic terrain");
}

fn advance_seconds(run: &mut Run, seconds: f64) {
    let end = crate::spell_cast::now(run.app.world()) + seconds;
    while crate::spell_cast::now(run.app.world()) + 1e-6 < end {
        run.app.update();
    }
}

fn owned_wyverns(run: &mut Run) -> Vec<Entity> {
    let owner = run.actor;
    let world = run.app.world_mut();
    let mut query = world.query::<(Entity, &Npc, &SummonedCreature)>();
    query
        .iter(world)
        .filter(|(_, npc, summon)| {
            npc.template_id == WYVERN_ENTRY
                && summon.owner == owner
                && summon.spell_id == WYVERN_SUMMON
        })
        .map(|(entity, _, _)| entity)
        .collect()
}

fn native_duration(run: &Run, spell: u32) -> f64 {
    let info = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap();
    f64::from(info.misc.as_ref().unwrap().duration.unwrap().duration_ms) / 1000.0
}

fn spawn_native_wyvern(run: &mut Run) -> Entity {
    let owner = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, WYVERN_SUMMON, Some(owner), true)
        .expect("native summon child admission includes authored destination geometry");
    run.app.update();
    let summons = owned_wyverns(run);
    assert_eq!(
        summons.len(),
        1,
        "real effect28 must create one authentic owned native entity"
    );
    summons[0]
}

fn cry_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == WYVERN_CRY && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "native Howl real summon closure/lifetime control; prepared253/current world required"]
fn dps_parity_howl_native_child_uses_authentic_entity_and_native_lifetime() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_howl_player();
    let duration = native_duration(&run, WYVERN_SUMMON);
    let actor = spawn_native_wyvern(&mut run);
    let expiry = run
        .app
        .world()
        .get::<SummonedCreature>(actor)
        .unwrap()
        .expires_at
        .unwrap();
    assert!(expiry > crate::spell_cast::now(run.app.world()));
    advance_seconds(&mut run, duration + 0.1);
    assert!(
        run.app.world().get_entity(actor).is_err(),
        "actual native duration must retire the entity"
    );
}

#[test]
#[ignore = "native Howl Cry earned from actual summon; not an apply-aura fixture"]
fn dps_parity_howl_native_real_wyvern_earns_db2_cry_stacks() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_howl_player();
    assert_eq!(cry_stacks(&run), 0);
    let expected = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(HOWL)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .base_points;
    let actor = spawn_native_wyvern(&mut run);
    assert!(run.app.world().get_entity(actor).is_ok());
    assert_eq!(
        cry_stacks(&run),
        expected as u8,
        "real selected Howl summon earns native root effect2 stack count"
    );
}

#[test]
#[ignore = "native actual summon without selected Howl must not grant Cry"]
fn dps_parity_howl_native_removed_passive_keeps_real_summon_without_cry() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_howl_player();
    let owner = run.actor;
    let removed = crate::auras::remove_auras(
        run.app.world_mut(),
        owner,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == HOWL,
    );
    assert_eq!(removed.len(), 1);
    spawn_native_wyvern(&mut run);
    assert_eq!(cry_stacks(&run), 0);
}

fn prepare_howl_austin_at_native_spawn() -> (Run, Entity) {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    load_native_collision(&mut run);
    let at = *run
        .app
        .world()
        .get::<shared::components::Position>(run.actor)
        .unwrap();
    // Choose the fixture target's initial spawn beside the existing actor; no
    // Position component is rewritten and native visibility remains authoritative.
    let target = crate::zones::summon_creature(
        run.app.world_mut(),
        run.actor,
        DUMMY_ENTRY,
        [
            at.x,
            -(at.z + NEARBY_FIXTURE_TARGET_OFFSET),
            at.y,
            std::f32::consts::PI,
        ],
    )
    .unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::creature_scripts::TrainingDummy>(target)
            .is_some()
    );
    for retired in run.targets.drain(..) {
        crate::zones::unsummon(run.app.world_mut(), retired);
    }
    run.targets = vec![target];
    run.damage_targets = vec![target];
    run.target_waves = vec![
        json!({"at": crate::spell_cast::now(run.app.world()), "entities": [target.to_bits()]}),
    ];
    let after = run
        .app
        .world()
        .get::<shared::components::Position>(run.actor)
        .unwrap();
    assert_eq!(
        [after.x, after.y, after.z],
        [at.x, at.y, at.z],
        "fixture construction must not teleport Austin"
    );
    (run, pet)
}

#[test]
#[ignore = "native initial-spawn LOS falsifier; no teleport or visibility override"]
fn dps_parity_howl_native_initial_target_spawn_passes_real_admission() {
    let _seed = crate::random::seed_scope(70_448);
    let (mut run, _) = prepare_howl_austin_at_native_spawn();
    let sample = record_native_admission(&mut run, "native-nearby-initial-spawn");
    eprintln!("NATIVE_HOWL_NEARBY_SPAWN {sample}");
    for result in sample["admission"].as_array().unwrap() {
        assert!(
            result["admitted"] == true,
            "same actor must pass native KC/Cobra admission at the new target's initial spawn: {sample}"
        );
    }
}

fn focus(run: &Run) -> f32 {
    run.app
        .world()
        .get::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap()
        .get(shared::components::PowerType::Focus)
        .unwrap()
        .current
}

fn record_native_admission(run: &mut Run, label: &str) -> Value {
    let actor = run.actor;
    let target = run.targets[0];
    let events_before = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let focus_before = focus(run);
    let results: Vec<_> = [KILL_COMMAND, 193455]
        .into_iter()
        .map(|id| {
            let info = run
                .app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(id)
                .unwrap()
                .clone();
            let result = crate::spell_cast::read_native_cast_start(
                run.app.world_mut(),
                actor,
                &info,
                target,
            );
            json!({"spell": id, "admitted": result.is_ok(), "result": format!("{result:?}")})
        })
        .collect();
    assert_eq!(
        focus(run),
        focus_before,
        "eligibility reads must not spend or grant Focus"
    );
    assert_eq!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len(),
        events_before,
        "eligibility reads must not cast or emit combat events"
    );
    let world = run.app.world();
    json!({"label": label, "time": crate::spell_cast::now(world), "admission": results,
        "actor": actor.to_bits(), "target": target.to_bits(), "focus": focus_before,
        "actor_position": world.get::<shared::components::Position>(actor).map(|p| [p.x, p.y, p.z]),
        "target_position": world.get::<shared::components::Position>(target).map(|p| [p.x, p.y, p.z]),
        "actor_model": world.get::<shared::components::ModelDisplay>(actor).map(|model| model.display_id),
        "target_model": world.get::<shared::components::ModelDisplay>(target).map(|model| model.display_id),
        "pet": crate::hunter_pet::summoned_pet(world, actor).map(Entity::to_bits),
        "kill_commands": kill_command_casts(run)})
}

#[test]
#[ignore = "read-only native KC/Cobra admission geometry diagnostic, not gameplay RED"]
fn dps_parity_howl_native_root_admission_records_real_geometry() {
    let _seed = crate::random::seed_scope(70_448);
    let (mut run, _) = super::power_tests::prepare_hunter_and_pet();
    load_native_collision(&mut run);
    let mut samples = vec![record_native_admission(&mut run, "before-engage")];
    engage(&mut run);
    samples.push(record_native_admission(&mut run, "engaged-before-update"));
    for (label, seconds) in [
        ("first-update", 0.05),
        ("one-second", 0.95),
        ("five-seconds", 4.0),
        ("thirty-five-seconds", 30.0),
    ] {
        advance_seconds(&mut run, seconds);
        samples.push(record_native_admission(&mut run, label));
    }
    eprintln!(
        "NATIVE_HOWL_ADMISSION {}",
        serde_json::to_string(&samples).unwrap()
    );
    let directory =
        std::path::PathBuf::from(std::env::var_os("DPS_HOWL_OUTPUT").expect("DPS_HOWL_OUTPUT"));
    let output = directory.join("root-admission.json");
    assert!(!output.exists(), "do not overwrite diagnostic evidence");
    std::fs::write(output, serde_json::to_vec_pretty(&json!({"qualification": "read-only eligibility diagnostic, not gameplay RED", "samples": samples})).unwrap()).unwrap();
}

fn kill_command_casts(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(KILL_COMMAND)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        })
        .count()
}

fn observed_howl_actions(run: &Run) -> Vec<u32> {
    use shared::protocol::CombatLogKind;
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.source == Some(run.actor.to_bits()))
        .filter_map(|event| match event.spell_id {
            Some(WYVERN_SUMMON | BEAR_SUMMON) if event.kind == CombatLogKind::Summon => {
                event.spell_id
            }
            Some(BOAR_IMPACT) if event.kind == CombatLogKind::CastSuccess => event.spell_id,
            _ => None,
        })
        .take(3)
        .collect()
}

#[test]
#[ignore = "native clock/KC, real Wyvern and Bear summons, literal Boar action"]
fn dps_parity_howl_native_first_cycle_preserves_sourced_action_order() {
    let _seed = crate::random::seed_scope(70_448);
    let (mut run, _) = prepare_howl_austin_at_native_spawn();
    engage(&mut run);
    let bound = native_duration(&run, HOWL_COOLDOWN) * 4.0;
    advance_seconds(&mut run, bound);
    assert!(
        kill_command_casts(&run) > 0,
        "actual ordinary KC prerequisite"
    );
    assert_eq!(
        observed_howl_actions(&run),
        vec![WYVERN_SUMMON, BOAR_IMPACT, BEAR_SUMMON],
        "actual summon logs and native Boar action must follow Wyvern/Boar/Bear"
    );
}

#[test]
#[ignore = "native ordinary combat without selected Howl cannot start its cycle"]
fn dps_parity_howl_native_removed_root_has_no_combat_cycle() {
    let _seed = crate::random::seed_scope(70_448);
    let (mut run, _) = prepare_howl_austin_at_native_spawn();
    let owner = run.actor;
    crate::auras::remove_spell(run.app.world_mut(), owner, HOWL, RemoveReason::Cancelled);
    engage(&mut run);
    let bound = native_duration(&run, HOWL_COOLDOWN) * 2.0;
    advance_seconds(&mut run, bound);
    assert!(
        kill_command_casts(&run) > 0,
        "actual ordinary KC prerequisite"
    );
    assert!(!crate::auras::has_aura(
        run.app.world(),
        owner,
        HOWL_COOLDOWN
    ));
    assert_eq!(cry_stacks(&run), 0);
    assert!(observed_howl_actions(&run).is_empty());
}

#[test]
#[ignore = "native combat clock to ready to ordinary KC and actual summoned Wyvern"]
fn dps_parity_howl_native_combat_readiness_is_consumed_by_real_kill_command() {
    let _seed = crate::random::seed_scope(70_448);
    let (mut run, pet) = prepare_howl_austin_at_native_spawn();
    let owner = run.actor;
    assert_eq!(
        crate::hunter_pet::summoned_pet(run.app.world(), owner),
        Some(pet)
    );
    assert!(crate::auras::has_aura(run.app.world(), owner, HOWL));
    engage(&mut run);
    let end = crate::spell_cast::now(run.app.world()) + native_duration(&run, HOWL_COOLDOWN) * 2.0;
    let mut seen_ready = false;
    let mut consumed_by_kc = false;
    while crate::spell_cast::now(run.app.world()) < end {
        let ready = crate::auras::has_aura(run.app.world(), owner, WYVERN_READY);
        seen_ready |= ready;
        let casts_before = kill_command_casts(&run);
        run.app.update();
        if ready
            && !crate::auras::has_aura(run.app.world(), owner, WYVERN_READY)
            && kill_command_casts(&run) > casts_before
        {
            consumed_by_kc = !owned_wyverns(&mut run).is_empty();
            break;
        }
    }
    assert!(
        kill_command_casts(&run) > 0,
        "actual Austin must execute ordinary KC, not remain idle"
    );
    assert!(
        seen_ready,
        "native combat entry and DB2 cooldown must earn first ready aura"
    );
    assert!(
        consumed_by_kc,
        "ordinary KC consumes readiness only with the real Wyvern entity"
    );
    assert_eq!(owned_wyverns(&mut run).len(), 1);
    assert!(cry_stacks(&run) > 0, "Cry follows the real summoned actor");
    assert!(
        crate::auras::has_aura(run.app.world(), owner, HOWL_COOLDOWN),
        "consumption rearms native cooldown"
    );
}
