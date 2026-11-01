//! Follower safety accounts for Doom explosions on already-marked enemies.
use super::follower_dps_diagnostic::*;
use super::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_demonology_doom_reach_covers_remote_marked_targets() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 266, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    let mut position = app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap()
        .clone();
    position.x += 30.0;
    *app.world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap() = position;
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 460553, Some(target), 1);
    assert!(crate::auras::has_aura(app.world(), target, 460553));
    let damage = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(460555)
        .unwrap();
    let radius = crate::spell_cast::targets::radius(
        damage
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap(),
        1,
    );
    assert!(radius > 0.0);
    let hand =
        crate::followers::spell_unintended_reach(app.world(), actor, 105174, target).unwrap();
    assert!(
        hand.0 >= 30.0 + radius,
        "Hand of Gul'dan must cover a remote Doom explosion: {hand:?}"
    );
    let demonbolt =
        crate::followers::spell_unintended_reach(app.world(), actor, 264178, target).unwrap();
    assert!(
        demonbolt.1 >= radius,
        "Demonbolt's earned Doom can later explode around its target: {demonbolt:?}"
    );
}
