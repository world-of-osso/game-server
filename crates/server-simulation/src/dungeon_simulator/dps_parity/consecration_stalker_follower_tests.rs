//! Uninteractible Consecration summons do not pull idle enemies.
use super::follower_dps_diagnostic::prepare_class_follower;
use super::*;
use shared::components::{Npc, Position};

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn consecration_stalker_near_idle_enemy_does_not_enter_combat() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, dummy, _) = prepare_class_follower(app, copy, 70, 90);
    app.world_mut().despawn(dummy);
    app.world_mut()
        .entity_mut(actor)
        .remove::<server_follower_apl::FollowerApl>();
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    app.world_mut().entity_mut(owner).insert(Position {
        x: -20.0,
        y: 0.0,
        z: 0.0,
    });
    let enemy =
        crate::zones::summon_creature(app.world_mut(), actor, 243983, [28.5, 0.0, 0.0, 0.0])
            .expect("idle Tarasek");
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 26573, Some(actor), 1);
    let stalker = app
        .world_mut()
        .query::<(Entity, &Npc, &crate::zones::TempSummon)>()
        .iter(app.world())
        .find(|(_, npc, summon)| npc.template_id == 43499 && summon.summoner == actor)
        .map(|(unit, _, _)| unit)
        .expect("actual Consecration stalker");
    for _ in 0..100 {
        app.update();
        assert!(
            crate::combat_refs::active_opponents(app.world(), enemy)
                .next()
                .is_none(),
            "idle enemy entered combat"
        );
        assert!(
            crate::combat_refs::active_opponents(app.world(), stalker)
                .next()
                .is_none(),
            "stalker entered combat"
        );
        assert!(
            !matches!(
                app.world()
                    .get::<crate::creature_ai::CreatureState>(stalker),
                Some(crate::creature_ai::CreatureState::Pursuing { .. })
            ),
            "stalker pursued an enemy"
        );
    }
}
