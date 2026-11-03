//! Compare the production follower APL against real creature templates in one arena.
use super::follower_dps_diagnostic::{prepare_class_follower, start_fixed_time_fight};
use super::*;
use server_follower_apl::literal_rotation::DecisionTrace;
use shared::components::{Health, Level, Npc, UnitFactionTemplate, UnitFlags, UnitLevel};
use shared::protocol::CombatLogKind;

const LEYMOR: u32 = 186644;
const SECONDS: u32 = 30;

struct Outcome {
    decisions: Vec<(String, Option<usize>, String)>,
    damage: i64,
}

fn measure(spec: u32, entry: u32) -> Outcome {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("native world snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    assert_ne!(copy.map_id, 2651, "the Candle King mechanic is absent");
    let (mut app, actor, original, _) = prepare_class_follower(app, copy, spec, 90);
    let world = app.world_mut();
    let faction = world.get::<UnitFactionTemplate>(original).unwrap().clone();
    let flags = world.get::<UnitFlags>(original).unwrap().clone();
    let display = world
        .get::<shared::components::ModelDisplay>(original)
        .unwrap()
        .clone();
    let scale = world
        .get::<shared::components::UnitScale>(original)
        .unwrap()
        .clone();
    let armor = world
        .get::<crate::level_scaling::CreatureArmor>(original)
        .unwrap()
        .0;
    crate::zones::unsummon(world, original);
    let target = crate::zones::summon_creature(world, actor, entry, [1.5, 0.0, 0.0, 0.0])
        .expect("real target creature template must spawn");
    assert_eq!(world.get::<Npc>(target).unwrap().template_id, entry);
    // Template-specific spawn rolls are setup, not combat RNG.
    let _combat_seed = crate::random::seed_scope(70448);
    world.entity_mut(target).insert((
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
        Level(90),
        UnitLevel(90),
        faction,
        flags,
        display,
        scale,
        crate::level_scaling::CreatureArmor(armor),
        crate::scripted_ai::ReactState::Passive,
        shared::threat::ThreatTable::default(),
    ));
    world
        .entity_mut(target)
        .remove::<shared::level_scaling::LevelScaling>()
        .remove::<crate::spell_cast::powers::CreatureHealthRegenTimer>()
        .remove::<crate::scripted_ai::CreatureScriptName>()
        .remove::<crate::scripted_ai::CreatureAiState>()
        .remove::<crate::creature_scripts::TrainingDummy>()
        .remove::<crate::creature_aggro::AggroRange>()
        .remove::<crate::creature_ai::CreatureState>();
    assert!(
        world
            .get::<crate::scripted_ai::CreatureScriptName>(target)
            .is_none()
    );
    assert!(
        world
            .get::<crate::scripted_ai::CreatureAiState>(target)
            .is_none()
    );
    assert!(
        world
            .get::<crate::creature_scripts::TrainingDummy>(target)
            .is_none()
    );
    world.entity_mut(actor).insert(DecisionTrace::default());
    for _ in 0..5 {
        app.update();
    }
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        start_fixed_time_fight(&mut app, owner, actor, target, f64::from(SECONDS));
    }));
    if result.is_err() {
        println!(
            "precombat fixture state: {}",
            report::engagement(app.world(), actor, &[target])
        );
        panic!("spec{spec} entry{entry}: production precombat failed");
    }
    for _ in 0..SECONDS * 20 {
        app.update();
    }
    let world = app.world();
    let decisions = world.get::<DecisionTrace>(actor).unwrap().0.clone();
    let damage = world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
        })
        .map(|event| i64::from(event.amount.max(0)))
        .sum();
    Outcome { decisions, damage }
}

fn compare(spec: u32) {
    let dummy = measure(spec, DUMMY_ENTRY);
    let control = measure(spec, DUMMY_ENTRY);
    let boss = measure(spec, LEYMOR);
    assert_eq!(
        dummy.decisions, control.decisions,
        "spec{spec} dummy control decision sequence differs"
    );
    assert_eq!(
        dummy.damage, control.damage,
        "spec{spec} dummy control damage differs"
    );
    println!(
        "spec{spec} dummy dps={} boss dps={} dummy decisions={} boss decisions={}",
        dummy.damage as f64 / f64::from(SECONDS),
        boss.damage as f64 / f64::from(SECONDS),
        dummy.decisions.len(),
        boss.decisions.len()
    );
    assert!(
        !dummy.decisions.is_empty(),
        "spec{spec} made no APL decisions"
    );
    assert_eq!(
        dummy.decisions, boss.decisions,
        "spec{spec} APL decision sequence differs"
    );
    assert_eq!(
        dummy.damage, boss.damage,
        "spec{spec} effective DPS differs"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB and real Leymor template"]
fn ret_rotation_does_not_depend_on_boss_identity() {
    compare(70);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB and real Leymor template"]
fn arcane_rotation_does_not_depend_on_boss_identity() {
    compare(62);
}
