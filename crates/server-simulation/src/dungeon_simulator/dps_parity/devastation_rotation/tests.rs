//! Focused native leaf proof, not89-row controller or300s parity qualification.
use super::super::literal_rotation::equipment::Equipment;
use super::super::literal_rotation::execution::{Execution, NativeAction, Prepared};
use super::super::literal_rotation::{Driver, capture_fixture};
use super::super::{Run, prepare};
use super::{bindings, facts::DevFacts};
use crate::simc_apl_native::{Encounter, FightStyle};
use server::simc_apl::{Action, ActionFacts, Program};
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

const LIVING_FLAME: u32 = 361469;
const LIVING_FLAME_DAMAGE: u32 = 361500;
const FIRE_BREATH_DAMAGE: u32 = 357209;
const ETERNITY_SURGE_DAMAGE: u32 = 359077;
const AZURE_STRIKE: u32 = 362969;

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
        if run.app.world().get::<CastState>(run.actor).is_none()
            && landed_damage(&run, LIVING_FLAME_DAMAGE)
        {
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
        landed_damage(&run, LIVING_FLAME_DAMAGE),
        "native Living Flame must hit the selected target"
    );
}

#[test]
#[ignore = "native Dev source target_if with self-target Fire Breath empower"]
fn literal_dev_binding_native_fire_breath_preserves_enemy_context_and_self_cast() {
    let mut run = prepared();
    super::super::literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    run.app.update();
    let mut bindings = bindings::build();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [NativeAction {
            name: "fire_breath",
            spell: bindings.actions["fire_breath"],
            talent: None,
            self_target: true,
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
    let probe = Action {
        name: "fire_breath".into(),
        target: None,
        options: [("empower_to".into(), "1".into())].into(),
    };
    assert!(
        prepared.execution(&probe, &equipment).unwrap().is_some(),
        "ordinary native self-target Fire Breath must already be ready"
    );
    assert_eq!(
        facts.target_ids(&probe).unwrap(),
        vec![run.targets[0].to_bits()],
        "native enemy target eligibility must precede shared selection"
    );
    // Unmodified rank1 fb_sc source row; this leaf does not run the other88 rows.
    let source = "fire_breath,empower_to=1,target_if=max:target.health.pct,if=active_enemies=1|active_enemies>4";
    let mut driver = Driver::new(Program::parse(&[("native_leaf", source)]).unwrap());
    let (action, execution) = driver
        .select("native_leaf", &facts, &prepared, &equipment)
        .unwrap()
        .expect(
            "native source Fire Breath must select an enemy context despite its self cast target",
        );
    assert_eq!(action.target, Some(run.targets[0].to_bits()));
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..6000 {
        run.app.update();
        if run.app.world().get::<CastState>(run.actor).is_none() {
            break;
        }
    }
    assert!(run.app.world().get::<CastState>(run.actor).is_none());
    let world = run.app.world();
    let aura = world
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FIRE_BREATH_DAMAGE && aura.caster == run.actor)
        .expect("source-selected enemy must receive native Fire Breath");
    let spell =
        crate::spell_cast::scripts::empower_evoker::build_fire_breath_release(world, run.actor, 1);
    assert_eq!(
        aura.duration_ms,
        crate::auras::fresh_duration_ms(world, run.actor, run.targets[0], &spell)
    );
}

#[test]
#[ignore = "native exact rank1 Eternity Surge source leaf; full controller remains open"]
fn literal_dev_binding_native_eternity_surge_selects_and_hits_the_real_enemy() {
    let mut run = prepared();
    let mut bindings = bindings::build();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [NativeAction {
            name: "eternity_surge",
            spell: bindings.actions["eternity_surge"],
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
    let source = "eternity_surge,empower_to=1,target_if=max:target.health.pct";
    let mut driver = Driver::new(Program::parse(&[("native_leaf", source)]).unwrap());
    let (action, execution) = driver
        .select("native_leaf", &facts, &prepared, &equipment)
        .unwrap()
        .expect("native source rank1 Eternity Surge must select its actual enemy");
    assert_eq!(action.target, Some(run.targets[0].to_bits()));
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..6000 {
        run.app.update();
        if run.app.world().get::<CastState>(run.actor).is_none()
            && landed_damage(&run, ETERNITY_SURGE_DAMAGE)
        {
            break;
        }
    }
    assert!(run.app.world().get::<CastState>(run.actor).is_none());
    assert!(
        landed_damage(&run, ETERNITY_SURGE_DAMAGE),
        "native rank1 release must damage the selected enemy"
    );
}

#[test]
#[ignore = "native unconditional Azure Strike source filler; full controller remains open"]
fn literal_dev_binding_native_azure_strike_source_filler_hits_the_real_enemy() {
    let mut run = prepared();
    let mut bindings = bindings::build();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [NativeAction {
            name: "azure_strike",
            spell: bindings.actions["azure_strike"],
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
    let mut driver = Driver::new(Program::parse(&[("native_leaf", "azure_strike")]).unwrap());
    let (action, execution) = driver
        .select("native_leaf", &facts, &prepared, &equipment)
        .unwrap()
        .expect("native source Azure Strike filler must remain selectable");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..6000 {
        run.app.update();
        if landed_damage(&run, AZURE_STRIKE) {
            break;
        }
    }
    assert!(
        landed_damage(&run, AZURE_STRIKE),
        "native source filler must damage the selected enemy"
    );
}

fn landed_damage(run: &Run, spell: u32) -> bool {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            let actors_spell =
                event.source == Some(run.actor.to_bits()) && event.spell_id == Some(spell);
            let target_took_damage = event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0;
            actors_spell && target_took_damage
        })
}
