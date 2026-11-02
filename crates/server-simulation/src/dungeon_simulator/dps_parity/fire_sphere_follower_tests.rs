use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::ActiveAuras;
use shared::components::Health;

fn sphere(app: &App, actor: Entity) -> Option<(u8, u32)> {
    app.world().get::<ActiveAuras>(actor)?.iter()
        .find(|aura| aura.spell_id == 448604 && aura.caster == actor)
        .map(|aura| (aura.stacks, aura.duration_ms))
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn fire_follower_spheres_survive_combat_exit() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 63, 90);
    let owner = app.world().get::<crate::followers::Follower>(actor).unwrap().owner;
    for _ in 0..5 { app.update(); }
    assert_eq!(sphere(&app, actor), None, "a fresh follower starts without free spheres");
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for _ in 0..2400 {
        if sphere(&app, actor).is_some() { break; }
        app.update();
    }
    let earned = sphere(&app, actor).expect("follower earned a sphere during combat");
    assert_eq!(earned.1, 604_800_000, "sphere has the native seven-day duration");
    app.world_mut().get_mut::<Health>(target).unwrap().current = 0.0;
    crate::combat::combat_stop(app.world_mut(), owner);
    crate::combat::combat_stop(app.world_mut(), actor);
    for _ in 0..200 { app.update(); }
    assert_eq!(crate::combat_refs::active_opponents(app.world(), actor).count(), 0);
    assert_eq!(sphere(&app, actor), Some(earned), "earned spheres survive combat exit and follower regroup");
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn fire_fixed_fight_fixture_starts_with_three_spheres() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, _, _) = prepare_class_follower(app, copy, 63, 90);
    assert_eq!(sphere(&app, actor), None);
    super::fire_rotation::apply_initial_spheres(app.world_mut(), actor);
    assert_eq!(sphere(&app, actor), Some((3, 604_800_000)));
}
