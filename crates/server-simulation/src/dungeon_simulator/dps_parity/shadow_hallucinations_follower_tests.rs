use super::*;
use super::follower_dps_diagnostic::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_shadow_hallucinations_restricts_insanity_to_listed_combat_casts() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 8092, Some(target), false).unwrap();
    for _ in 0..80 {
        app.update();
    }
    let gains = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| event.kind == CombatLogKind::Energize && event.spell_id == Some(199579))
            .map(|event| event.amount)
            .sum::<i32>()
    };
    assert_eq!(gains(&app), 0, "Mind Blast must not trigger Hallucinations");
    crate::spell_cast::creature_cast(app.world_mut(), actor, 17, Some(actor), false).unwrap();
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        gains(&app),
        600,
        "Power Word: Shield grants native Hallucinations Insanity"
    );
}

