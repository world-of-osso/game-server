//! SimC76f8118 rogue4551–4561: literal operands retain each Garrote's actual snapshot.
use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle};
use server::simc_apl::{Action, ActionFacts};

fn read_garrote_factors(
    run: &Run,
    bindings: &Bindings,
    equipment: &literal_rotation::equipment::Equipment,
    target: Entity,
) -> [f64; 2] {
    let facts = assassination_rotation::RogueFacts::new(
        run.app.world(),
        run.actor,
        bindings,
        equipment,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        &run.targets,
    );
    let action = Action {
        name: "garrote".into(),
        source_row: None,
        target: Some(target.to_bits()),
        options: Default::default(),
    };
    ["pmultiplier", "dot.garrote.pmultiplier"].map(|name| facts.value_for(&action, name).unwrap())
}

fn cast_garrote(run: &mut Run, target: Entity) {
    let actor = run.actor;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(703, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_garrote_snapshot_preserves_empowerment_after_window_and_scopes_real_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 3).unwrap();
    let actor = run.actor;
    let strong = run.targets[0];
    let ordinary = run.targets[1];
    let absent = run.targets[2];
    let mut bindings = Bindings {
        actions: [("garrote".into(), 703)].into(),
        buffs: [("garrote".into(), 703)].into(),
        ..Default::default()
    };
    let equipment =
        literal_rotation::equipment::Equipment::load(run.app.world_mut(), actor, &mut bindings)
            .unwrap();
    let absent_factors = read_garrote_factors(&run, &bindings, &equipment, absent);
    engage(&mut run);
    cast_garrote(&mut run, strong);
    cast_garrote(&mut run, ordinary);
    let bonus = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 392401 && aura.caster == actor)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    let empowered = f64::from(1.0 + bonus / 100.0);
    assert!(empowered > 1.0, "actual native Improved Garrote bonus");
    for _ in 0..140 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), actor, 392401));
    assert!(crate::auras::has_aura(run.app.world(), strong, 703));
    assert_eq!(
        read_garrote_factors(&run, &bindings, &equipment, strong),
        [empowered; 2],
        "the real empowered bleed keeps its factor after the post-stealth window ends"
    );
    assert_eq!(
        absent_factors, [0.0; 2],
        "an absent target-owned bleed has no snapshot"
    );
    assert!(crate::auras::has_aura(run.app.world(), ordinary, 703));
    cast_garrote(&mut run, ordinary);
    assert_eq!(
        read_garrote_factors(&run, &bindings, &equipment, ordinary),
        [1.0; 2],
        "an ordinary refresh replaces that bleed's earlier empowered factor"
    );
    assert_eq!(
        read_garrote_factors(&run, &bindings, &equipment, strong),
        [empowered; 2],
        "reading the secondary target must not rewrite or substitute the primary snapshot"
    );
}
