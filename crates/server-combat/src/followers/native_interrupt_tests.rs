//! Native Skull Bash charge and interrupt behavior.
use super::{kit_tests::level32, utility_tests};
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{ModelDisplay, Position, UnitScale};
use shared::protocol::CombatLogKind;

#[derive(Debug, PartialEq)]
struct MovementIntent {
    state: Option<crate::creature_ai::CreatureState>,
    follow: Option<crate::creature_ai::FollowOwner>,
    target: Option<Option<Entity>>,
}

fn read_movement_intent(h: &Harness, unit: Entity) -> MovementIntent {
    MovementIntent {
        state: h
            .app
            .world()
            .get::<crate::creature_ai::CreatureState>(unit)
            .copied(),
        follow: h
            .app
            .world()
            .get::<crate::creature_ai::FollowOwner>(unit)
            .copied(),
        target: h
            .app
            .world()
            .get::<crate::networking::CombatTarget>(unit)
            .map(|target| target.0),
    }
}

fn prepare_native_charge_target(h: &mut Harness) {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (display_id, scale): (u32, f32) = db.query_row("SELECT CreatureDisplayID,DisplayScale FROM content_creature_template_model WHERE CreatureID=6 ORDER BY Idx LIMIT 1", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[6]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((ModelDisplay { display_id }, UnitScale(scale)));
}

/// Each ordinary cast rolls its own native child221514 defense. Compare motion
/// to that execution's outcome, not to a second independent random roll.
fn assert_native_skull_bash_motion(h: &Harness, unit: Entity, before: Position) {
    let after = *h.app.world().get::<Position>(unit).unwrap();
    let missed = h.inbox.log.iter().any(|event| {
        event.source == Some(unit.to_bits())
            && event.spell_id == Some(221514)
            && matches!(event.kind, CombatLogKind::Miss(_))
    });
    if missed {
        assert_eq!(after, before, "a defended charge must not move its caster");
        return;
    }
    assert_ne!(after, before, "an undefended native charge must move");
    let target = h.app.world().get::<Position>(h.npc).unwrap();
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), h.npc)
        .expect("native model reach, not a guessed default");
    let distance =
        Vec3::new(after.x, after.y, after.z).distance(Vec3::new(target.x, target.y, target.z));
    assert!(
        (distance - reach).abs() < 0.001,
        "native charge stops at the target's source reach"
    );
}

#[test]
fn native_skull_bash_charge_preserves_navigation_and_cooldown() {
    let (mut native, native_unit) = level32(232800);
    prepare_native_charge_target(&mut native);
    let native_enemy = native.npc;
    utility_tests::prepare_enemy_spec(&mut native, native_enemy, 63);
    crate::spell_cast::creature_cast(
        native.app.world_mut(),
        native_enemy,
        133,
        Some(native_unit),
        false,
    )
    .unwrap();
    let native_before = *native.app.world().get::<Position>(native_unit).unwrap();
    let native_intent = read_movement_intent(&native, native_unit);
    crate::spell_cast::creature_cast(
        native.app.world_mut(),
        native_unit,
        106839,
        Some(native_enemy),
        false,
    )
    .unwrap();
    assert_eq!(read_movement_intent(&native, native_unit), native_intent);
    native.advance(20);
    assert_native_skull_bash_motion(&native, native_unit, native_before);
    assert!(
        native
            .inbox
            .go
            .iter()
            .any(|event| event.caster == native_unit.to_bits() && event.spell_id == 106839)
    );
    assert!(matches!(
        crate::spell_cast::creature_cast(
            native.app.world_mut(),
            native_unit,
            106839,
            Some(native_enemy),
            false
        ),
        Err(crate::spell_cast::CastFailure::OnCooldown)
    ));
}
