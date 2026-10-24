use super::super::literal_rotation::{self, Driver, equipment::Equipment, execution::Prepared};
use super::{bindings, facts::EleFacts, source};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle};
use bevy::prelude::*;
use server::simc_apl::{ActionFacts, Facts, Program};

fn fixture(count: usize) -> (super::super::Run, Bindings, Equipment, Encounter) {
    let mut run = super::super::prepare(262, count).unwrap();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    super::super::engage_fixture_opponents(&mut run);
    let mut bindings = bindings::build();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    (run, bindings, equipment, encounter)
}

fn apply(world: &mut World, caster: Entity, target: Entity, spell: u32) {
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
#[ignore = "Elemental native target-fact qualification pending"]
fn literal_ele_native_owned_target_facts_select_and_damage_the_foreign_dot_target() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let (mut run, bindings, equipment, encounter) = fixture(3);
    let other = run
        .app
        .world()
        .get::<crate::followers::Follower>(run.actor)
        .unwrap()
        .owner;
    apply(run.app.world_mut(), run.actor, run.targets[0], 188389);
    apply(run.app.world_mut(), other, run.targets[1], 188389);
    let mut driver = Driver::new(
        Program::parse(&[(
            "default",
            "lightning_bolt,target_if=min:dot.flame_shock.remains",
        )])
        .unwrap(),
    );
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        bindings::execution_actions().filter(|action| action.name == "lightning_bolt"),
        false,
    )
    .unwrap();
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert_eq!(facts.value("active_dot.flame_shock").unwrap(), 1.0);
    let (action, execution) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .unwrap();
    assert_eq!(action.target, Some(run.targets[1].to_bits()));
    assert_eq!(
        facts.value_for(&action, "dot.flame_shock.remains").unwrap(),
        0.0
    );
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..80 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let root_hits = |target: Entity| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(188196)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    assert!(root_hits(run.targets[1]) > 0);
    assert_eq!(root_hits(run.targets[0]), 0);

    apply(run.app.world_mut(), run.actor, run.targets[0], 197209);
    apply(run.app.world_mut(), other, run.targets[1], 197209);
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        bindings::execution_actions().filter(|action| action.name == "lightning_bolt"),
        false,
    )
    .unwrap();
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert_eq!(facts.value("lightning_rod").unwrap(), 1.0);
    let mut driver = Driver::new(
        Program::parse(&[(
            "default",
            "lightning_bolt,target_if=min:debuff.lightning_rod.remains",
        )])
        .unwrap(),
    );
    let (action, _) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .unwrap();
    assert_eq!(action.target, Some(run.targets[1].to_bits()));
    assert_eq!(
        facts
            .value_for(&action, "debuff.lightning_rod.remains")
            .unwrap(),
        0.0
    );
    assert!(facts.value("dot.flame_shock.typo").is_err());
    assert!(facts.value("debuff.lightning_rod.typo").is_err());
    let invalid = server::simc_apl::Action {
        name: "lightning_bolt".into(),
        target: Some(Entity::PLACEHOLDER.to_bits()),
        options: Default::default(),
    };
    assert!(
        facts
            .value_for(&invalid, "dot.flame_shock.remains")
            .is_err()
    );
    let unknown = Program::parse(&[(
        "default",
        "lightning_bolt,if=0&talent.unknown_elemental_talent",
    )])
    .unwrap();
    assert!(
        unknown
            .select("default", &facts, &mut unknown.state(), |_| Ok(true))
            .is_err()
    );
    assert!(source::load().is_ok());
}

#[test]
#[ignore = "Elemental native refresh/expiry qualification pending"]
fn literal_ele_native_refreshability_and_counts_follow_real_aura_time() {
    let (mut run, bindings, equipment, encounter) = fixture(1);
    apply(run.app.world_mut(), run.actor, run.targets[0], 188389);
    let maximum =
        crate::auras::maximum_duration_ms(run.app.world(), run.actor, run.targets[0], 188389)
            .unwrap();
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert_eq!(facts.value("active_dot.flame_shock").unwrap(), 1.0);
    assert_eq!(facts.value("dot.flame_shock.refreshable").unwrap(), 0.0);
    let ticks = (f64::from(maximum) * 0.7 / 50.0).ceil() as usize + 2;
    for _ in 0..ticks {
        run.app.update();
    }
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert!(facts.value("dot.flame_shock.remains").unwrap() > 0.0);
    assert_eq!(facts.value("dot.flame_shock.refreshable").unwrap(), 1.0);
    for _ in 0..(maximum as usize / 50 + 2) {
        run.app.update();
    }
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert_eq!(facts.value("active_dot.flame_shock").unwrap(), 0.0);
    assert_eq!(facts.value("dot.flame_shock.remains").unwrap(), 0.0);
    assert_eq!(facts.value("dot.flame_shock.refreshable").unwrap(), 1.0);
}
