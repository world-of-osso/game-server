//! A paid follower Bite earns Sabertooth on the actual target.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_bite_applies_sabertooth_to_owned_rip() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    assert!(
        crate::auras::has_aura(app.world(), actor, 202031),
        "native Sabertooth selected"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    let rip = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1079)
        .unwrap()
        .clone();
    let before = crate::auras::damage_taken_multiplier(app.world(), target, actor, &rip);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::ComboPoints, 5.0);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 22568, Some(target), false).unwrap();
    for _ in 0..10 {
        app.update();
    }
    let aura = app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 391722 && a.caster == actor)
        .expect("landed Bite earns owned Sabertooth");
    assert_eq!(aura.stacks, 5, "five paid points give five stacks");
    let after = crate::auras::damage_taken_multiplier(app.world(), target, actor, &rip);
    assert!(
        (after / before - 1.30).abs() < 0.001,
        "five stacks increase owned Rip by 30%: {before} -> {after}"
    );
    let other = crate::auras::damage_taken_multiplier(app.world(), target, target, &rip);
    assert!(
        (other - before).abs() < 0.001,
        "another caster cannot use this debuff"
    );
}
