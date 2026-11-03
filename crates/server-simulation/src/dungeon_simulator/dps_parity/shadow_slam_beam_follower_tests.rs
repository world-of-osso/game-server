//! Tentacle Slam's native target area and caster-to-target line constrain pull safety.
use super::follower_dps_diagnostic::prepare_class_follower;
use super::*;
use shared::components::Position;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn shadow_slam_line_blocks_idle_enemy_inside_but_not_outside() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, primary, _) = prepare_class_follower(app, copy, 258, 90);
    app.world_mut().entity_mut(primary).insert(Position {
        x: 20.0,
        y: 0.0,
        z: 0.0,
    });
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::networking::CombatTarget(Some(primary)));
    let spawn = |app: &mut App, x: f32, z: f32| {
        crate::zones::summon_creature(app.world_mut(), actor, 243983, [x, -z, 0.0, 0.0])
            .expect("idle Tarasek")
    };
    let outside = spawn(&mut app, 10.0, 14.0);
    let beyond = spawn(&mut app, 35.0, 0.0);
    assert!(
        crate::followers::action_safe(app.world(), actor, primary, Some(1227280)),
        "idle enemies beyond the beam and target area do not block Slam"
    );
    let inside = spawn(&mut app, 10.0, 0.0);
    assert!(
        !crate::followers::action_safe(app.world(), actor, primary, Some(1227280)),
        "an idle enemy on the line blocks Slam"
    );
    let mut damage = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1227621)
        .unwrap()
        .clone();
    damage.cast_destination = Some([20.0, 0.0, 0.0]);
    let effect = damage.effects[0].clone();
    let selected = crate::spell_cast::targets::select_targets(
        app.world_mut(),
        actor,
        Some(primary),
        &damage,
        &effect,
        0,
        false,
    )
    .unwrap();
    assert!(selected.contains(&inside), "native Slam hits the line");
    assert!(
        selected.contains(&primary),
        "native Slam hits the target area"
    );
    assert!(
        !selected.contains(&outside),
        "native Slam does not hit outside the line"
    );
    assert!(
        !selected.contains(&beyond),
        "native Slam stops at its destination"
    );
}
