//! Native strikes reduce actual running Brew cooldowns.
use super::*;

fn fortifying_remaining(run: &Run) -> u64 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(115203)
        .unwrap();
    world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .remaining_cooldown_ms(spell, crate::spell_cast::history_now(world))
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn selected_salsalabim_keg_smash_resets_breath_of_fire() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(run.app.world().get::<crate::class_progression::TraitLoadout>(actor)
        .unwrap().active_talent_spells().any(|talent| talent.spell_id == 383697));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115181, Some(target), false).unwrap();
    for _ in 0..40 { run.app.update(); }
    let remaining = |run: &Run| {
        let world = run.app.world();
        let spell = world.resource::<crate::spell_info::SpellStore>().get(115181).unwrap();
        world.get::<crate::spell_cast::SpellHistory>(actor).unwrap()
            .remaining_cooldown_ms(spell, crate::spell_cast::history_now(world))
    };
    assert!(remaining(&run) > 0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false).unwrap();
    assert_eq!(remaining(&run), 0, "Keg Smash resets Breath of Fire");
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn tiger_palm_and_keg_smash_reduce_brew_cooldowns() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115203, Some(actor), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let before = fortifying_remaining(&run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
        .unwrap();
    assert!(
        (1499..=1501).contains(&(before - fortifying_remaining(&run))),
        "Tiger Palm plus selected Face Palm must reduce the running Brew by 1.5s"
    );
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 228563, Some(actor), 0);
    assert!(crate::auras::has_aura(run.app.world(), actor, 228563));
    let before = fortifying_remaining(&run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
        .unwrap();
    assert!(
        (3999..=4001).contains(&(before - fortifying_remaining(&run))),
        "Keg Smash with Blackout Combo and selected Meditative Focus must reduce the running Brew by 4s"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 228563),
        "Keg Smash consumes Blackout Combo"
    );
}
