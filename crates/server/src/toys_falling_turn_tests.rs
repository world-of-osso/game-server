//! TC a352b1fa SpellAuraEffects.cpp2969–2983, Unit.cpp13933–13961:
//! aura409 owns movement bit39 until the last such aura is removed.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, apply_aura};
use shared::components::{FeatherFall, PlayerMotion};

const CAN_TURN_WHILE_FALLING: u64 = 0x0000_0080_0000_0000;

fn load_source_spells(h: &mut Harness) {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(PlayerMotion(PlayerMotion::FALLING));
}

fn assert_turning_toy_expires(item: u32, spell: u32, duration_frames: usize) {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("falling-turn.redb"));
    load_source_spells(&mut h);
    learn_effect_toy(&mut h, character, item);
    assert_eq!(activate(&mut h, item)[0].error, None);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    let owner = h.caster;
    assert_eq!(
        h.app.world().get::<FeatherFall>(owner),
        Some(&FeatherFall(true))
    );
    assert!(
        h.app
            .world()
            .get::<PlayerMotion>(owner)
            .unwrap()
            .contains(CAN_TURN_WHILE_FALLING | PlayerMotion::FALLING)
    );
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(owner)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .unwrap();
    assert!(aura.effects.iter().any(|effect| effect.aura_type == 409));
    h.advance(duration_frames - 20);
    assert!(crate::auras::has_aura(h.app.world(), owner, spell));
    assert!(
        h.app
            .world()
            .get::<PlayerMotion>(owner)
            .unwrap()
            .contains(CAN_TURN_WHILE_FALLING)
    );
    h.advance(40);
    assert!(!crate::auras::has_aura(h.app.world(), owner, spell));
    let motion = h.app.world().get::<PlayerMotion>(owner).unwrap();
    assert!(!motion.contains(CAN_TURN_WHILE_FALLING));
    assert!(motion.contains(PlayerMotion::FALLING));
    assert_eq!(
        h.app.world().get::<FeatherFall>(owner),
        Some(&FeatherFall(false))
    );
}

#[test]
fn toy_coverage_turn_falling_skyhorn_source_use_and_sixty_second_expiry() {
    assert_turning_toy_expires(131811, 196768, 1200);
}

#[test]
fn toy_coverage_turn_falling_plume_source_use_and_ten_second_expiry() {
    assert_turning_toy_expires(182729, 340077, 200);
}

#[test]
fn toy_coverage_turn_falling_removing_one_native_aura_retains_other_owner() {
    let mut h = harness(Vec::new(), &[196768, 340077], 8).unwrap();
    load_source_spells(&mut h);
    let owner = h.caster;
    for spell in [196768, 340077] {
        apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: owner,
                target: owner,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    assert!(
        h.app
            .world()
            .get::<PlayerMotion>(owner)
            .unwrap()
            .contains(CAN_TURN_WHILE_FALLING)
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        owner,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 340077,
    );
    assert!(
        h.app
            .world()
            .get::<PlayerMotion>(owner)
            .unwrap()
            .contains(CAN_TURN_WHILE_FALLING)
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        owner,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 196768,
    );
    assert_eq!(
        h.app.world().get::<PlayerMotion>(owner),
        Some(&PlayerMotion(PlayerMotion::FALLING))
    );
}
