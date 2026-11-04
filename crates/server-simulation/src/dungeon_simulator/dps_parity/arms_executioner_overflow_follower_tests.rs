use super::*;
use crate::auras::ActiveAuras;

fn stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 445584 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires frozen native Arms profile and selected traits"]
fn arms_executioner_overflow_keeps_earlier_and_new_expiries() {
    let mut run = prepare_literal(71, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 1270718));
    let start = crate::spell_cast::history_now(run.app.world());
    for _ in 0..6 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 445584, Some(actor), 1);
        for _ in 0..20 {
            run.app.update();
        }
    }
    assert_eq!(stacks(&run), 5, "the sixth application is capped");
    while crate::spell_cast::history_now(run.app.world()) < start + 18.1 {
        run.app.update();
    }
    assert_eq!(
        stacks(&run),
        4,
        "the first application expires at 18 seconds"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 445584, Some(actor), 1);
    while crate::spell_cast::history_now(run.app.world()) < start + 22.1 {
        run.app.update();
    }
    assert_eq!(stacks(&run), 1, "the capped application remains scheduled");
    while crate::spell_cast::history_now(run.app.world()) < start + 23.1 {
        run.app.update();
    }
    assert_eq!(stacks(&run), 0, "the capped sixth application also expires");
}
