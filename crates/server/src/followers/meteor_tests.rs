//! TC a352b1fa spell_mage.cpp1544–1595; native70448 Meteor chain.
use super::rotation_dummy_tests::{prepare_dummies, prepare_follower};
use bevy::prelude::*;
use shared::components::{Level, UnitLevel};
use shared::protocol::CombatLogKind;

const METEOR: u32 = 153561;
const IMPACT: u32 = 351140;
const BURN: u32 = 155158;

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
fn fire_meteor_native70_delay_burn_entry_exit_and_expiry_use_native_clocks() {
    use shared::components::Position;
    let (mut h, caster, targets) = prepare_meteor(3);
    for target in &targets {
        h.app
            .world_mut()
            .entity_mut(*target)
            .insert((Level(70), UnitLevel(70)));
    }
    let outside = targets[2];
    h.app.world_mut().get_mut::<Position>(outside).unwrap().x += 20.0;
    let start = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, METEOR, Some(targets[0]), false)
        .unwrap();
    h.advance(79);
    assert!(
        !h.inbox.log[start..].iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(IMPACT)
                && event.kind == CombatLogKind::Damage
        }),
        "native3s field plus1s missile cannot impact at3950ms"
    );
    h.advance(1);
    assert!(
        h.inbox.log[start..].iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(IMPACT)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "native4s impact deadline"
    );
    h.advance(10);
    assert!(
        h.inbox.log[start..].iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(BURN)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        }),
        "native500ms burn tick, not a synthetic damage pulse"
    );
    assert!(
        !h.inbox.log[start..].iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(outside.to_bits())
                && matches!(event.spell_id, Some(IMPACT | BURN))
                && event.kind == CombatLogKind::Damage
        }),
        "authored8yd area excludes20yd target"
    );
    let mover = targets[1];
    let origin = *h.app.world().get::<Position>(mover).unwrap();
    h.app.world_mut().get_mut::<Position>(mover).unwrap().x += 20.0;
    h.advance(1);
    assert!(
        !crate::auras::has_aura(h.app.world(), mover, BURN),
        "sourceOnExit removes the burn"
    );
    h.app.world_mut().entity_mut(mover).insert(origin);
    h.advance(1);
    assert!(
        crate::auras::has_aura(h.app.world(), mover, BURN),
        "sourceOnEnter reapplies on return"
    );
    h.advance(68);
    for target in &targets[..2] {
        assert!(
            !crate::auras::has_aura(h.app.world(), *target, BURN),
            "native4s burn field expiry removes remaining aura"
        );
    }
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
