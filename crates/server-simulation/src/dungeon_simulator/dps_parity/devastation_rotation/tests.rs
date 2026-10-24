//! Focused native leaf proof, not89-row controller or300s parity qualification.
use super::super::literal_rotation::equipment::Equipment;
use super::super::literal_rotation::execution::{NativeAction, Prepared};
use super::super::literal_rotation::{Driver, capture_fixture};
use super::super::{Run, prepare};
use super::{bindings, facts::DevFacts};
use crate::simc_apl_native::{Encounter, FightStyle};
use server::simc_apl::{Action, ActionFacts, Program};
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

const LIVING_FLAME: u32 = 361469;
const LIVING_FLAME_DAMAGE: u32 = 361500;

fn prepared() -> Run {
    let mut run = prepare(1467, 1).unwrap();
    let clock = encounter(&run);
    capture_fixture(run.app.world_mut(), run.actor, clock).unwrap();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
    run.app.update();
    for _ in 0..6000 {
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(LIVING_FLAME)
            .unwrap();
        let gcd = crate::spell_cast::native_apl::gcd_value(world, run.actor, spell, "gcd.remains")
            .unwrap();
        if gcd == 0.0 && world.get::<CastState>(run.actor).is_none() {
            return run;
        }
        run.app.update();
    }
    panic!("native Living Flame fixture never became ready");
}

fn encounter(run: &Run) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

#[test]
#[ignore = "native Dev binding/Living Flame leaf; full89-row controller remains open"]
fn literal_dev_binding_native_living_flame_uses_actual_target_and_context() {
    let mut run = prepared();
    let mut bindings = bindings::build();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let action = Action {
        name: "living_flame".into(),
        target: None,
        options: Default::default(),
    };
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [NativeAction {
            name: "living_flame",
            spell: bindings.actions["living_flame"],
            talent: None,
            self_target: false,
        }],
        false,
    )
    .unwrap();
    let facts = DevFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter(&run),
        &run.targets,
    );
    assert_eq!(
        facts.target_ids(&action).unwrap(),
        vec![run.targets[0].to_bits()]
    );
    assert!(facts.value_for(&action, "execute_time").unwrap() > 0.0);
    assert_eq!(
        facts.value_for(&action, "target.health.pct").unwrap(),
        100.0
    );
    let mut driver = Driver::new(Program::parse(&[("native_leaf", "living_flame")]).unwrap());
    let (action, execution) = driver
        .select("native_leaf", &facts, &prepared, &equipment)
        .unwrap()
        .unwrap();
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..6000 {
        run.app.update();
        if run.app.world().get::<CastState>(run.actor).is_none() && living_flame_hit(&run) {
            break;
        }
    }
    assert!(run.app.world().get::<CastState>(run.actor).is_none());
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let target = run.targets[0].to_bits();
    let actor = run.actor.to_bits();
    assert!(events.iter().any(|event| event.source == Some(actor)
        && event.target == Some(target)
        && event.spell_id == Some(LIVING_FLAME)
        && event.kind == CombatLogKind::CastSuccess));
    assert!(
        living_flame_hit(&run),
        "native Living Flame must hit the selected target"
    );
}

fn living_flame_hit(run: &Run) -> bool {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(LIVING_FLAME_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
}
