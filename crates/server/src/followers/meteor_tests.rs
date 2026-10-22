//! TC a352b1fa spell_mage.cpp1544–1595; native70448 Meteor chain.
use super::rotation_dummy_tests::{prepare_dummies, prepare_follower};
use bevy::prelude::*;
use shared::components::{Level, UnitLevel};
use shared::protocol::CombatLogKind;

const METEOR: u32 = 153561;
const IMPACT: u32 = 351140;

fn prepare_meteor(
    count: usize,
) -> (
    crate::spell_cast::test_harness::Harness,
    Entity,
    Vec<Entity>,
) {
    let (mut h, caster) = prepare_follower(209059, 63, 70);
    let path = std::env::var_os("FOLLOWER_KIT_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::test_data::world_db_path);
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app
        .insert_resource(crate::spell_cast::area_trigger::load_aura_field_catalog(&db).unwrap());
    let targets = prepare_dummies(&mut h, count);
    (h, caster, targets)
}

#[test]
fn fire_meteor_native70_single_target_kit_admits_authored_meteor() {
    let (mut h, caster, targets) = prepare_meteor(1);
    let target = targets[0];
    h.app
        .world_mut()
        .entity_mut(target)
        .insert((Level(70), UnitLevel(70)));
    let start = h.inbox.go.len();
    for _ in 0..200 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.inbox.go[start..]
            .iter()
            .any(|event| { event.caster == caster.to_bits() && event.spell_id == METEOR }),
        "native selected Meteor must be admitted on its authored single-target APL, not blanket AoEmin3"
    );
}

#[test]
fn fire_meteor_native70_ordinary_three_target_cast_deals_delayed_impact_damage() {
    let (mut h, caster, targets) = prepare_meteor(3);
    for target in &targets {
        h.app
            .world_mut()
            .entity_mut(*target)
            .insert((Level(70), UnitLevel(70)));
    }
    let start = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, METEOR, Some(targets[0]), false)
        .unwrap();
    h.advance(100);
    assert!(
        h.inbox.log[start..].iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(METEOR)
                && event.kind == CombatLogKind::CastSuccess
        }),
        "ordinary root must execute before testing scripted impact"
    );
    for target in targets {
        assert!(
            h.inbox.log[start..].iter().any(|event| {
                event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(IMPACT)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
            "ordinary Meteor must reach actual native351140 damage on target{target:?}"
        );
    }
}
