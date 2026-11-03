use super::*;
use crate::auras;
use crate::spell_info::SpellStore;

#[test]
#[ignore = "requires selected MID2 Protection profile and world data"]
fn dps_parity_protection_devastator_moves_focus_and_boosts_execute() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(73, 2).unwrap();
    let actor = run.actor;
    let first = run.targets[0];
    let second = run.targets[1];
    assert!(auras::has_aura(run.app.world(), actor, 1264330));
    engage(&mut run);
    let execute = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(260798)
        .unwrap()
        .clone();
    let shield = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(23922)
        .unwrap()
        .clone();
    let baseline = auras::damage_taken_multiplier(run.app.world(), first, actor, &execute);
    let shield_before = auras::damage_taken_multiplier(run.app.world(), first, actor, &shield);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 236282, Some(first), 1);
    for _ in 0..10 {
        run.app.update();
    }
    assert!(
        auras::has_aura(run.app.world(), first, 1277983),
        "real Devastator grants focus"
    );
    let focused = auras::damage_taken_multiplier(run.app.world(), first, actor, &execute);
    let shield_after = auras::damage_taken_multiplier(run.app.world(), first, actor, &shield);
    assert!(
        (focused / baseline - 1.3).abs() < 0.001,
        "Execute factor {baseline} -> {focused}"
    );
    assert!(
        (shield_after / shield_before - 1.0).abs() < 0.001,
        "Shield Slam is unaffected"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 236282, Some(second), 1);
    for _ in 0..10 {
        run.app.update();
    }
    assert!(!auras::has_aura(run.app.world(), first, 1277983));
    assert!(auras::has_aura(run.app.world(), second, 1277983));
}
