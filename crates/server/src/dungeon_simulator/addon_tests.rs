//! Real Map34 template- and spawn-addon intake, including ordinary evade restoration.
use super::runtime::{build_app, create_copy, spawn_population};
use shared::components::{Npc, Position};

fn assert_native_spawn_and_return_addon(guid: u32, entry: u32, spell: u32) {
    let mut app = build_app();
    let copy = create_copy(&mut app);
    let actors = spawn_population(&mut app, copy);
    let actor = actors[&guid];
    assert_eq!(app.world().get::<Npc>(actor).unwrap().template_id, entry);
    assert!(
        app.world()
            .get::<crate::zones::SpawnAddonAuras>(actor)
            .unwrap()
            .0
            .contains(&spell)
    );
    let home = *app.world().get::<Position>(actor).unwrap();
    app.update();
    assert!(
        crate::auras::has_aura(app.world(), actor, spell),
        "native entry{entry}/guid{guid} must receive source addon{spell} at spawn"
    );
    crate::creature_aggro::enter_evade_mode(app.world_mut(), actor);
    assert!(
        !crate::auras::has_aura(app.world(), actor, spell),
        "native evade must remove this non-stays-after-combat addon"
    );
    app.update();
    app.update();
    assert_eq!(*app.world().get::<Position>(actor).unwrap(), home);
    assert_eq!(
        app.world().get::<crate::creature_ai::CreatureState>(actor),
        Some(&crate::creature_ai::CreatureState::Idle)
    );
    assert!(
        crate::auras::has_aura(app.world(), actor, spell),
        "native home arrival must restore source addon{spell} on entry{entry}"
    );
    assert!(
        app.world().get::<crate::zones::AddonAuras>(actor).is_none(),
        "ordinary addon runner must consume the restoration marker"
    );
}

#[test]
fn simulator_native_addon_46263_template19818_at_spawn_and_return() {
    assert_native_spawn_and_return_addon(20375675, 46263, 19818);
}

#[test]
fn simulator_native_addon_46381_template10022_at_spawn_and_return() {
    assert_native_spawn_and_return_addon(20375718, 46381, 10022);
}

#[test]
fn simulator_native_addon_46405_spawn55474_at_spawn_and_return() {
    assert_native_spawn_and_return_addon(20375779, 46405, 55474);
}
