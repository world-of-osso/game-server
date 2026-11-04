use super::*;
use crate::auras::ActiveAuras;

fn frenzy_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 335082 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires frozen native Fury profile and selected traits"]
fn fury_frenzy_capped_refresh_preserves_old_and_incoming_expiries() {
    let mut run = prepare_literal(72, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 335077));
    let start = crate::spell_cast::history_now(run.app.world());
    for _ in 0..11 {
        // The real final Rampage execution earns Frenzy through its native producer.
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 201363, Some(target), 1);
        for _ in 0..20 {
            run.app.update();
        }
    }
    assert_eq!(frenzy_stacks(&run), 10, "the eleventh grant is capped");
    while crate::spell_cast::history_now(run.app.world()) < start + 12.1 {
        run.app.update();
    }
    assert_eq!(
        frenzy_stacks(&run),
        9,
        "the first grant still expires at12s"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 201363, Some(target), 1);
    while crate::spell_cast::history_now(run.app.world()) < start + 21.1 {
        run.app.update();
    }
    assert_eq!(frenzy_stacks(&run), 1, "the capped grant remains scheduled");
    while crate::spell_cast::history_now(run.app.world()) < start + 22.1 {
        run.app.update();
    }
    assert_eq!(
        frenzy_stacks(&run),
        0,
        "the capped grant also loses a stack"
    );
}
