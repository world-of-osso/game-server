//! Wild Guardian is earned from Incarnation and consumed by the replacement cast.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_wild_action_requires_and_consumes_incarnation_override() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    engage(&mut run);
    let available = |run: &Run| {
        crate::spell_cast::knowledge::resolve_action(run.app.world(), actor, 1269658).unwrap()
    };
    assert_eq!(available(&run), None);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
        .unwrap();
    assert_eq!(available(&run), Some(1269658));
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1269658, Some(actor), false)
        .unwrap();
    assert_eq!(available(&run), None);
    run.app.update();
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 1308176 && a.caster == actor)
    );
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::Rage,
        1000.0,
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        actor,
        400254,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    assert!(
        !run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 1308176 && a.caster == actor)
    );
    assert!(
        run.app
            .world_mut()
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(run.app.world())
            .any(|s| s.owner == actor && s.spell_id == 1308399)
    );
}
