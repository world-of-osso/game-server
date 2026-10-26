//! Native player storage and generic summon targeting/dispatch prerequisites.
use super::*;
use shared::components::Npc;

fn actual_owned_actors(run: &mut Run, entry: u32) -> Vec<Entity> {
    run.app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature, &Npc)>()
        .iter(run.app.world())
        .filter(|(_, summon, npc)| summon.owner == run.actor && npc.template_id == entry)
        .map(|(entity, _, _)| entity)
        .collect()
}

#[test]
#[ignore = "requires private267 native player profile"]
fn pet_prereq_native_player_completes_its_real_main_imp_summon() {
    let _seed = crate::random::seed_scope(70448);
    let run = prepare_measurement(267, 1).unwrap();
    let imp = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("real precombat688 must create its native main pet, not only log cast success");
    assert_eq!(run.app.world().get::<Npc>(imp).unwrap().template_id, 416);
    assert!(
        run.app
            .world()
            .get::<shared::components::UnitIsPet>(imp)
            .unwrap()
            .0
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
}

#[test]
#[ignore = "requires private267 native player profile"]
fn pet_prereq_native_infernal_value_missile_creates_its_actual_child_actor() {
    // Native1122 E2/type148 supplies the destination and value to111685.
    // TC a352b1fa SpellEffects.cpp725–788: one native child, retained costs.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    let actual = literal_rotation::execution::resolve_action(run.app.world(), run.actor, 1122)
        .unwrap()
        .expect("selected native Infernal action");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(actual, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    let actors = actual_owned_actors(&mut run, 89);
    assert_eq!(
        actors.len(),
        1,
        "actual1122 must dispatch148 into exactly one native111685/89 actor"
    );
}

#[test]
#[ignore = "requires private267 native player profile"]
fn pet_prereq_native_overfiend_destination85_creates_the_actual_temporary_actor() {
    // TC a352b1fa Spell.cpp1690–1738/SharedDefines.h3044: generic destination
    //85 is front-left of the current destination; CheckDst supplies caster origin.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(434587)
        .unwrap()
        .clone();
    let launched = crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &spell,
        Some(run.actor),
        1,
    );
    if !launched {
        // Only diagnose a rejected cast: never replan or reroll a successful one.
        let failure = crate::spell_cast::plan_triggered_spell(
            run.app.world_mut(),
            run.actor,
            &spell,
            Some(run.actor),
            1,
        )
        .err();
        panic!("native Overfiend admission must accept destination85: {failure:?}");
    }
    for _ in 0..5 {
        run.app.update();
    }
    let actors = actual_owned_actors(&mut run, 217429);
    assert_eq!(
        actors.len(),
        1,
        "native background434587 must create one actual Overfiend"
    );
}
