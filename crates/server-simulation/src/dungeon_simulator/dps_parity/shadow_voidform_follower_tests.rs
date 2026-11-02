use super::follower_dps_diagnostic::*;
use super::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_shadow_voidform_suspends_and_restores_existing_shadowform() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    for selected in [true, false] {
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2515).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 70);
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::Follower>();
        for _ in 0..4 {
            app.update();
        }
        assert!(!crate::auras::has_aura(app.world(), actor, 232698));
        if selected {
            crate::spell_cast::creature_cast(app.world_mut(), actor, 232698, Some(actor), false)
                .unwrap();
            for _ in 0..40 {
                app.update();
            }
            assert!(crate::auras::has_aura(app.world(), actor, 232698));
        }
        crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 228260, Some(target), false)
            .unwrap();
        for _ in 0..30 {
            app.update();
        }
        assert!(crate::auras::has_aura(app.world(), actor, 194249));
        assert!(
            !crate::auras::has_aura(app.world(), actor, 232698),
            "Voidform must not stack Shadowform damage"
        );
        crate::auras::remove_auras(
            app.world_mut(),
            actor,
            crate::auras::RemoveReason::Expired,
            |aura| aura.spell_id == 194249 && aura.caster == actor,
        );
        assert_eq!(
            crate::auras::has_aura(app.world(), actor, 232698),
            selected,
            "Voidform expiry restores only a previously active Shadowform"
        );
    }
}
