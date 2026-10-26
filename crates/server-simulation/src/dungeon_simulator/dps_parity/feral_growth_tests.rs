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
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let frenzy =
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 274837)
            .unwrap();
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

fn cast_native_spell_after_regeneration(run: &mut Run, spell: u32) {
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

