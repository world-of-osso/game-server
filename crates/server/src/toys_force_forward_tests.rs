//! Aura241: TC HandleForceMoveForward retains FORCE_MOVEMENT until the last aura leaves.
//! Grizzlesnout129965: real Northrend admission, source transform/speed and timed recovery.
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
fn toyfx63_grizzlesnout_real_learning_use_restores_identity_after_source_expiry() {
    use crate::auras::{ActiveAuras, AuraStatMods};
    use shared::components::{CastState, Health, ModelDisplay};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("grizzlesnout.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(UnitFlags2(1));
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, 129965);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    assert_eq!(
        activate(&mut h, 129965)[0].error.as_deref(),
        Some("Requires area group4446")
    );
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == 194807));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    place_toy_caster_on_source_map_area(&mut h, 571, 819918, [3217.333, 6917.333], 3537);
    assert_eq!(activate(&mut h, 129965)[0].error, None);
    assert_eq!(
        h.app.world().get::<CastState>(caster).unwrap().duration,
        1.5
    );
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    h.advance(31);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 194807));
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        9287
    );
    assert_eq!(
        h.app.world().get::<UnitFlags2>(caster),
        Some(&UnitFlags2(0x41))
    );
    assert_eq!(
        h.app
            .world()
            .get::<AuraStatMods>(caster)
            .unwrap()
            .movement_speed,
        3.5
    );
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 194807)
        .unwrap();
    assert_eq!(aura.duration_ms, 15_000);
    let expires_at = aura.expires_at.unwrap();
    let remaining = expires_at - crate::auras::aura_now(h.app.world());
    assert!(
        (14.0..=15.0).contains(&remaining),
        "authored15s remaining: {remaining}"
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == 194807 && cd.duration_ms == 600_000 && !cd.is_gcd)
    );
    assert_eq!(
        activate(&mut h, 129965)[0].error.as_deref(),
        Some("OnCooldown")
    );
    let remaining = expires_at - crate::auras::aura_now(h.app.world());
    h.advance((remaining / 0.05).ceil() as usize + 2);
    assert!(
        !h.app
            .world()
            .get::<ActiveAuras>(caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 194807)
    );
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(
        h.app.world().get::<UnitFlags2>(caster),
        Some(&UnitFlags2(1))
    );
    assert_eq!(
        h.app
            .world()
            .get::<AuraStatMods>(caster)
            .unwrap()
            .movement_speed,
        1.0
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(
        activate(&mut h, 129965)[0].error.as_deref(),
        Some("OnCooldown")
    );
}
