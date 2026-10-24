//! Existing native empower/replacement APIs, selected through shared execution.
use super::super::{Driver, capture_fixture};
use super::*;
use crate::dungeon_simulator::dps_parity::{Run, prepare};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle};
use crate::spell_info::SpellStore;
use server::simc_apl::Program;
use shared::casting::CastState;
use shared::components::PowerType;

const FIRE_BREATH: u32 = 357208;
const FIRE_DAMAGE: u32 = 357209;
const DISINTEGRATE: u32 = 356995;
const LIVING_FLAME: u32 = 361469;
const QUELL: u32 = 351338;

fn fixture() -> (Run, Equipment) {
    let mut run = prepare(1467, 1).unwrap();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
    run.app.update();
    // Profile initialization may start a real GCD; wait without clearing it.
    for _ in 0..6000 {
        if native_gcd_remaining(&run) == 0.0
            && run.app.world().get::<CastState>(run.actor).is_none()
        {
            break;
        }
        run.app.update();
    }
    assert_eq!(native_gcd_remaining(&run), 0.0);
    assert!(run.app.world().get::<CastState>(run.actor).is_none());
    let mut bindings = Bindings::default();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    (run, equipment)
}

fn actions() -> impl Iterator<Item = NativeAction<'static>> {
    [
        NativeAction {
            name: "fire_breath",
            spell: FIRE_BREATH,
            talent: None,
            self_target: true,
        },
        NativeAction {
            name: "disintegrate",
            spell: DISINTEGRATE,
            talent: None,
            self_target: false,
        },
        NativeAction {
            name: "living_flame",
            spell: LIVING_FLAME,
            talent: None,
            self_target: false,
        },
        NativeAction {
            name: "quell",
            spell: QUELL,
            talent: None,
            self_target: false,
        },
    ]
    .into_iter()
}

fn action(name: &str) -> Action {
    Action {
        name: name.into(),
        target: None,
        options: Default::default(),
    }
}

fn available_power(run: &Run, power: PowerType) -> f32 {
    crate::spell_cast::powers::power_value(run.app.world(), run.actor, power)
        .unwrap()
        .0
}

fn native_gcd_remaining(run: &Run) -> f64 {
    let world = run.app.world();
    let spell = world.resource::<SpellStore>().get(DISINTEGRATE).unwrap();
    crate::spell_cast::native_apl::gcd_value(world, run.actor, spell, "gcd.remains").unwrap()
}

fn wait_for_native_gcd(run: &mut Run) {
    for _ in 0..6000 {
        if native_gcd_remaining(run) == 0.0 {
            assert!(
                run.app.world().get::<CastState>(run.actor).is_some(),
                "real channel must still exist"
            );
            return;
        }
        run.app.update();
    }
    panic!("native GCD did not finish during Disintegrate");
}

#[test]
#[ignore = "native execution option RED: actual selected empower stage"]
fn literal_execution_native_empower_option_releases_stage_one_not_max_hold() {
    let (mut run, equipment) = fixture();
    let mut driver =
        Driver::new(Program::parse(&[("default", "fire_breath,empower_to=1")]).unwrap());
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        actions(),
        false,
    )
    .unwrap();
    let facts = std::collections::BTreeMap::<String, f64>::new(); // Row has no numeric operands.
    let (action, execution) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .unwrap();
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..6000 {
        run.app.update();
        if run.app.world().get::<CastState>(run.actor).is_none() {
            break;
        }
    }
    let world = run.app.world();
    assert!(world.get::<CastState>(run.actor).is_none());
    let dot = world
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == run.actor)
        .unwrap_or_else(|| {
            panic!(
                "empower released without target DoT: actor={:?}, target={:?}, events={:?}",
                world.get::<shared::components::Position>(run.actor),
                world.get::<shared::components::Position>(run.targets[0]),
                world
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0,
            )
        });
    let expected =
        crate::spell_cast::scripts::empower_evoker::build_fire_breath_release(world, run.actor, 1);
    let expected = crate::auras::fresh_duration_ms(world, run.actor, run.targets[0], &expected);
    assert_eq!(
        dot.duration_ms, expected,
        "source empower_to=1 must reach native release metadata"
    );
}

#[test]
#[ignore = "native replacement readiness RED: actual channel, GCD, target, power and replacement"]
fn literal_execution_native_replacement_preserves_admission_and_only_replaces_when_ready() {
    let (mut run, equipment) = fixture();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        DISINTEGRATE,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    let replacement = Prepared::read_interrupting_self(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        actions(),
        false,
    )
    .unwrap();
    let flame = action("living_flame");
    assert!(
        replacement.execution(&flame, &equipment).unwrap().is_none(),
        "native GCD remains binding"
    );
    wait_for_native_gcd(&mut run);
    let before = run.app.world().get::<CastState>(run.actor).unwrap().elapsed;
    let essence = available_power(&run, PowerType::Essence);
    assert!(
        essence < 3.0,
        "real native pool cannot afford another three-Essence Disintegrate"
    );
    let ordinary = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        actions(),
        false,
    )
    .unwrap();
    assert!(ordinary.execution(&flame, &equipment).unwrap().is_none());
    let invalid = Prepared::read_interrupting_self(
        run.app.world_mut(),
        run.actor,
        &[run.actor],
        &equipment,
        actions(),
        false,
    )
    .unwrap();
    assert!(
        invalid
            .execution(&action("quell"), &equipment)
            .unwrap()
            .is_none(),
        "native hostile target rules remain binding"
    );
    let replacement = Prepared::read_interrupting_self(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        actions(),
        false,
    )
    .unwrap();
    assert!(
        replacement
            .execution(&action("disintegrate"), &equipment)
            .unwrap()
            .is_none(),
        "actual insufficient Essence remains binding"
    );
    assert_eq!(
        run.app.world().get::<CastState>(run.actor).unwrap().elapsed,
        before,
        "readiness cannot cancel/advance current cast"
    );
    assert_eq!(
        available_power(&run, PowerType::Essence),
        essence,
        "readiness cannot spend power"
    );
    let execution = replacement
        .execution(&flame, &equipment)
        .unwrap()
        .expect("source-authorized replacement must be selectable");
    let mana = available_power(&run, PowerType::Mana);
    let mut driver = Driver::new(Program::parse(&[("default", "living_flame")]).unwrap());
    driver
        .execute(run.app.world_mut(), run.actor, flame, execution)
        .unwrap();
    let cast = run.app.world().get::<CastState>(run.actor).unwrap();
    assert_eq!(cast.spell_id, LIVING_FLAME);
    assert!(cast.elapsed < before);
    for _ in 0..6000 {
        run.app.update();
        if run.app.world().get::<CastState>(run.actor).is_none() {
            break;
        }
    }
    assert!(run.app.world().get::<CastState>(run.actor).is_none());
    assert!(
        available_power(&run, PowerType::Mana) < mana,
        "replacement pays its actual native Mana cost"
    );
}
