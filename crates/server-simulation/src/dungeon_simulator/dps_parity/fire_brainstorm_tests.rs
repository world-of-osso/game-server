//! SimC76f8118 sc_mage.cpp2154–2157: actual Hot Streak conversion grants Brainstorm.
use super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::KnownSpells;

fn has_owned_buff(run: &Run, spell: u32) -> bool {
    run.app.world().get::<ActiveAuras>(run.actor).is_some_and(|auras| {
        auras.iter().any(|aura| aura.spell_id == spell && aura.caster == run.actor)
    })
}

fn fire_blast(run: &mut Run) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(108853, run.targets[0], Some(run.targets[0])),
    ).expect("ordinary native Fire Blast");
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_brainstorm_actual_hot_streak_conversion_grants_selected_intellect() {
    let mut run = prepare_literal(63, 1).unwrap();
    assert!(run.app.world().get::<KnownSpells>(run.actor).unwrap().contains(461261));
    assert!(!has_owned_buff(&run, 461531));
    fire_blast(&mut run);
    assert!(has_owned_buff(&run, 48107), "first critical hit earns Heating Up");
    assert!(!has_owned_buff(&run, 461531), "Heating Up alone cannot grant Brainstorm");
    for _ in 0..12 { run.app.update(); }
    fire_blast(&mut run);
    assert!(has_owned_buff(&run, 48108), "second actual critical hit converts to Hot Streak");
    assert!(has_owned_buff(&run, 461531), "Hot Streak conversion must grant selected native Brainstorm");
}

#[test]
#[ignore = "requires exact Fire63 inputs and private70448 DB"]
fn fire_brainstorm_unselected_conversion_does_not_grant_intellect() {
    let mut run = prepare_literal(63, 1).unwrap();
    let known = run.app.world().get::<KnownSpells>(run.actor).unwrap().iter().filter(|spell| *spell != 461261).collect();
    run.app.world_mut().entity_mut(run.actor).insert(KnownSpells::fixture(known));
    fire_blast(&mut run);
    for _ in 0..12 { run.app.update(); }
    fire_blast(&mut run);
    assert!(has_owned_buff(&run, 48108));
    assert!(!has_owned_buff(&run, 461531));
}
