//! Production level-70 follower Scythe's Embrace Fury from Reap-family casts.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

const SCYTHES_EMBRACE: u32 = 1246558;

/// Fury after a native Reap-family cast that starts from an empty pool.
fn fury_after_cast(app: &mut App, actor: Entity, target: Entity, spell: u32) -> f32 {
    let current = crate::spell_cast::powers::power_value(app.world(), actor, PowerType::Fury)
        .expect("Devourer Fury pool")
        .0;
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Fury, -current)
        .expect("Devourer Fury pool");
    crate::spell_cast::cast_triggered(app.world_mut(), actor, spell, Some(target), 1);
    crate::spell_cast::powers::power_value(app.world(), actor, PowerType::Fury)
        .expect("Devourer Fury pool")
        .0
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_devourer_scythes_embrace_energizes_reap_cull_and_eradicate() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 1480, 70);
    assert!(
        app.world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .contains(SCYTHES_EMBRACE),
        "the level 70 follower selects Scythe's Embrace"
    );
    // SimC76f8118 sc_demon_hunter.cpp6003–6007,6145–6149: native1261679 effect1 is 10 Fury.
    for spell in [1226019, 1245453, 1225826] {
        assert_eq!(
            fury_after_cast(&mut app, actor, target, spell),
            10.0,
            "spell {spell} executes Scythe's Embrace energize"
        );
    }
    let spells: Vec<_> = app
        .world()
        .get::<crate::class_progression::KnownSpells>(actor)
        .unwrap()
        .iter()
        .filter(|&spell| spell != SCYTHES_EMBRACE)
        .collect();
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::class_progression::KnownSpells::from_baseline(spells));
    assert_eq!(
        fury_after_cast(&mut app, actor, target, 1226019),
        0.0,
        "an unselected talent grants no Fury"
    );
}
