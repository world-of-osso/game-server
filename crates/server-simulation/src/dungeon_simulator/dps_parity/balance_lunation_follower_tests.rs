use super::*;

fn remaining(run: &Run, spell_id: u32) -> u64 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .remaining_cooldown_ms(spell, crate::spell_cast::now(world))
}

#[test]
#[ignore = "requires native Balance102 WCL profile and frozen world data"]
fn balance_lunation_foreground_moonfire_reduces_fury_but_background_hits_do_not() {
    let mut run = prepare_literal(102, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 429539));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 202770, Some(target), false)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let before = remaining(&run, 202770);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 8921, Some(target), false)
        .unwrap();
    let after = remaining(&run, 202770);
    assert_eq!(
        before - after,
        1500,
        "native Lunation effect0 earns 1.5s reduction"
    );
    let before_background = remaining(&run, 202770);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 8921, Some(target), 1);
    assert_eq!(
        remaining(&run, 202770),
        before_background,
        "background Moonfire is not a foreground action"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 429539,
    );
    let before_without = remaining(&run, 202770);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 8921, Some(target), false)
        .unwrap();
    assert_eq!(
        remaining(&run, 202770),
        before_without,
        "no reduction without Lunation"
    );
}
