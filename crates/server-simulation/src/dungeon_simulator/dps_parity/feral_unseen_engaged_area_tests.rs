//! Unseen picks engaged primaries; Swipe still reaches idle units around them.
use super::follower_dps_diagnostic::prepare_class_follower;
use super::*;
use shared::components::Position;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn feral_bite_checks_swipe_area_around_each_engaged_candidate() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, primary, _) = prepare_class_follower(app, copy, 103, 90);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::networking::CombatTarget(Some(primary)));
    crate::combat_refs::set_in_combat_with(app.world_mut(), owner, primary);
    let candidate =
        crate::zones::summon_creature(app.world_mut(), actor, 243983, [15.0, 0.0, 0.0, 0.0])
            .unwrap();
    crate::combat_refs::set_in_combat_with(app.world_mut(), owner, candidate);
    let outsider =
        crate::zones::summon_creature(app.world_mut(), actor, 243983, [20.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert!(
        !crate::followers::action_safe(app.world(), actor, primary, Some(22568)),
        "an idle enemy in an engaged candidate's Swipe radius must block Bite"
    );
    let swipe = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1263908)
        .unwrap()
        .clone();
    let effect = swipe.effects[0].clone();
    let selected = crate::spell_cast::targets::select_targets(
        app.world_mut(),
        actor,
        Some(candidate),
        &swipe,
        &effect,
        0,
        false,
    )
    .unwrap();
    assert!(
        selected.contains(&outsider),
        "native Swipe really hits the idle enemy"
    );
    app.world_mut().entity_mut(outsider).insert(Position {
        x: -20.0,
        y: 0.0,
        z: 0.0,
    });
    assert!(
        crate::followers::action_safe(app.world(), actor, primary, Some(22568)),
        "an idle enemy far from every candidate must not be blocked by a blanket caster radius"
    );
    let party = crate::followers::party_unintended_reach(app.world_mut(), &[actor]);
    assert!(
        party[0].2 >= 8.0,
        "pull planning must retain Swipe's target area: {party:?}"
    );
}
