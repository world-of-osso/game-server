use super::follower_dps_diagnostic::*;
use super::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_shadow_slam_applies_pain_through_misery() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    for selected in [true, false] {
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2515).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 70);
        assert!(
            app.world()
                .get::<crate::class_progression::TraitLoadout>(actor)
                .unwrap()
                .active_talent_spells()
                .any(|spell| spell.spell_id == 238558)
        );
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::Follower>();
        crate::spell_cast::creature_cast(app.world_mut(), actor, 1227280, Some(target), false)
            .unwrap();
        if !selected {
            app.world_mut()
                .entity_mut(actor)
                .remove::<crate::class_progression::TraitLoadout>();
        }
        for _ in 0..20 {
            app.update();
        }
        assert!(crate::auras::has_aura(app.world(), target, 34914));
        assert_eq!(crate::auras::has_aura(app.world(), target, 589), selected);
    }
}
