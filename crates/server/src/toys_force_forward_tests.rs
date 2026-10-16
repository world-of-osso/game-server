//! Aura241: TC HandleForceMoveForward retains FORCE_MOVEMENT until the last aura leaves.
use super::*;
use shared::components::UnitFlags2;

#[test]
fn toyfx62_force_forward_preserves_other_flags_until_last_source_aura_is_removed() {
    use crate::auras::{AuraApplication, AuraSource, RemoveReason};
    let mut h = harness(Vec::new(), &[148626, 194807], 8).unwrap();
    let unit = h.caster;
    h.app.world_mut().entity_mut(unit).insert(UnitFlags2(1));
    for spell_id in [148626, 194807] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: unit,
                target: unit,
                spell_id,
                effect_mask: 1, // Both real source spells carry241 at EffectIndex0.
                source: AuraSource::Cast,
            },
        );
        assert_eq!(
            h.app.world().get::<UnitFlags2>(unit),
            Some(&UnitFlags2(0x41))
        );
    }
    crate::auras::remove_auras(h.app.world_mut(), unit, RemoveReason::Interrupted, |aura| {
        aura.spell_id == 148626
    });
    assert_eq!(
        h.app.world().get::<UnitFlags2>(unit),
        Some(&UnitFlags2(0x41))
    );
    crate::auras::remove_auras(h.app.world_mut(), unit, RemoveReason::Expired, |aura| {
        aura.spell_id == 194807
    });
    assert_eq!(h.app.world().get::<UnitFlags2>(unit), Some(&UnitFlags2(1)));
}

#[test]
fn toyfx62_ashhide_real_learning_use_preserves_source_mount_area_and_twenty_second_lifetime() {
    use shared::components::{Health, Mounted};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let (mut h, character) = setup(&directory.path().join("ashhide.redb"));
    h.app.insert_resource(shared_world_data(&path));
    learn_effect_toy(&mut h, character, 104329);
    // Real Northshire terrain refuses the Timeless Isle restriction before recovery.
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    assert_eq!(activate(&mut h, 104329)[0].error, None);
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        shared::spell_data::CastFailReason::IncorrectArea
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == 148626 && !cd.is_gcd)
    );
    h.inbox.failed.clear();
    place_toy_caster_on_source_map_area(&mut h, 870, 811105, [-550.0, -4816.0], 6757);
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(UnitFlags2(1));
    assert_eq!(activate(&mut h, 104329)[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.app
            .world()
            .get::<Mounted>(h.caster)
            .expect("authentic Ashhide mount body")
            .mount_display_id,
        51484
    );
    assert_eq!(
        h.app.world().get::<UnitFlags2>(h.caster),
        Some(&UnitFlags2(0x41))
    );
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .unwrap();
    assert!((mods.movement_speed - 2.5).abs() < 1e-6);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 148626)
        .unwrap();
    let expiry = aura.expires_at.unwrap();
    assert_eq!(aura.duration_ms, 20000);
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    assert!((19.0..=20.0).contains(&remaining));
    h.advance((remaining / 0.05).ceil() as usize + 2);
    assert!(h.app.world().get::<Mounted>(h.caster).is_none());
    assert_eq!(
        h.app.world().get::<UnitFlags2>(h.caster),
        Some(&UnitFlags2(1))
    );
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert!(
        activate(&mut h, 104329)[0].error.is_some(),
        "source sixty-second item recovery survives expiry"
    );
}
