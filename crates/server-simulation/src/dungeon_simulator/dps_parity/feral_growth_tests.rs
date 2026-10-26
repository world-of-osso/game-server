//! Selected Feral native periodic producers, independent of rotation admission.
use super::*;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_selected_thriving_growth_native_rip_rake_ticks_execute_vines() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = prepare_literal(103, 1).expect("selected native Feral profile");
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 439528),
        "the exact profile owns Thriving Growth"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let frenzy =
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 274837).unwrap();
    cast_native_spell_after_regeneration(&mut run, frenzy);
    for _ in 0..100 {
        run.app.update();
    }
    let combo = crate::spell_cast::powers::power_value(
        run.app.world(),
        actor,
        shared::components::PowerType::ComboPoints,
    )
    .unwrap()
    .0;
    assert_eq!(
        combo, 5.0,
        "the ordinary native Frenzy earns Rip's five points"
    );
    cast_native_spell_after_regeneration(&mut run, 1822);
    cast_native_spell_after_regeneration(&mut run, 1079);
    for _ in 0..800 {
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 40.0);
    let damage: u64 = combat["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["spell_id"] == 439531)
        .map(|row| row["damage"].as_u64().unwrap())
        .sum();
    assert!(
        damage > 0,
        "actual selected Rip/Rake ticks must execute native Bloodseeker Vines on the real target{target:?}"
    );
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_native_vines_applications_expire_independently() {
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    let read = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 439531 && aura.caster == actor)
            .cloned()
    };
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 439531, Some(target), 1);
    let first = read(&run).expect("native Vines application");
    assert_eq!(first.stacks, 1);
    let first_expiry = first.expires_at.unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 439531, Some(target), 1);
    let second = read(&run).expect("native Vines refresh");
    assert_eq!(second.stacks, 2);
    assert!(second.expires_at.unwrap() > first_expiry);
    while crate::spell_cast::now(run.app.world()) <= first_expiry + 0.1 {
        run.app.update();
    }
    assert_eq!(
        read(&run).expect("second application remains live").stacks,
        1,
        "a Vines refresh must not extend the first stack beyond its native lifetime"
    );
    while crate::spell_cast::now(run.app.world()) <= second.expires_at.unwrap() + 0.1 {
        run.app.update();
    }
    assert!(
        read(&run).is_none(),
        "both native application lifetimes have ended"
    );
}

pub(super) fn cast_native_spell_after_regeneration(run: &mut Run, spell: u32) {
    for _ in 0..600 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => {
                preparation::finish_pending_setup(run).unwrap();
                return;
            }
            Err(
                crate::spell_cast::CastFailure::NotEnoughPower(_)
                | crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(error) => panic!("ordinary native spell{spell} setup refused: {error:?}"),
        }
    }
    panic!("native spell{spell} never became ready within the fixture setup bound");
}
