//! Native picker contract. Planner/caller integration remains owned by9468.
use super::kit_tests::{at_level, level32};
use super::kit_utility_tests::wait_for_native_school_unlock;
use super::{Follower, utility_tests};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{ModelDisplay, Position, UnitScale};
use shared::protocol::CombatLogKind;

fn try_candidates(h: &mut Harness, unit: Entity, candidates: &[Entity]) -> bool {
    let profile = h.app.world().get::<Follower>(unit).unwrap().profile.clone();
    super::kit_interrupt::try_interrupt_candidates(h.app.world_mut(), unit, &profile, candidates)
}

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

#[test]
#[ignore = "RED: full-kit picker wastes Counterspell on a noninterruptible cast"]
fn follower_kit_interrupt_does_not_spend_on_noninterruptible_cast() {
    let (mut h, unit) = level32(209059);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    h.app
        .world_mut()
        .get_mut::<CastState>(enemy)
        .unwrap()
        .interruptible = false;
    assert!(!try_candidates(&mut h, unit, &[enemy]));
    h.advance(3);
    assert!(h.app.world().get::<CastState>(enemy).is_some());
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 2139)
    );
}

#[test]
#[ignore = "RED: ready Shield must follow a genuinely cooling Rebuke, before other actions"]
fn follower_kit_interrupt_rebuke_cooldown_uses_ready_shield_before_buffs() {
    let (mut h, unit) = level32(209057);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    assert!(try_candidates(&mut h, unit, &[enemy]));
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt && event.spell_id == Some(96231))
    );
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 31935)
    );
    wait_for_native_school_unlock(&mut h, 96231);
    utility_tests::start_enemy_fireball(&mut h, unit);
    assert!(try_candidates(&mut h, unit, &[enemy]));
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.source == Some(unit.to_bits())
                && event.spell_id == Some(31935))
    );
}

fn two_casting_enemies(h: &mut Harness, unit: Entity) -> [Entity; 2] {
    let range = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(2139)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .range
        .unwrap()
        .max_hostile;
    let unit_x = range * 3.0;
    h.app.world_mut().get_mut::<Position>(unit).unwrap().x = unit_x;
    let first = h.npc;
    utility_tests::prepare_enemy_spec(h, first, 63);
    h.app
        .world_mut()
        .entity_mut(first)
        .insert(crate::networking::CombatTarget(Some(h.caster)));
    crate::spell_cast::creature_cast(h.app.world_mut(), first, 133, Some(h.caster), false).unwrap();
    let second = spawn_npc(&mut h.app, unit_x + range / 2.0);
    utility_tests::prepare_enemy_spec(h, second, 63);
    h.app
        .world_mut()
        .entity_mut(second)
        .insert(crate::networking::CombatTarget(Some(unit)));
    crate::spell_cast::creature_cast(h.app.world_mut(), second, 133, Some(unit), false).unwrap();
    [first, second]
}

#[test]
#[ignore = "RED: picker must continue to the second source-ready caster without chasing"]
fn follower_kit_interrupt_uses_second_engaged_caster_without_chasing() {
    let (mut h, unit) = level32(209059);
    let candidates = two_casting_enemies(&mut h, unit);
    let position = *h.app.world().get::<Position>(unit).unwrap();
    let intent = read_movement_intent(&h, unit);
    assert!(try_candidates(&mut h, unit, &candidates));
    assert_eq!(read_movement_intent(&h, unit), intent);
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.source == Some(unit.to_bits())
                && event.target == Some(candidates[1].to_bits())
                && event.spell_id == Some(2139))
    );
    assert!(h.app.world().get::<CastState>(candidates[0]).is_some());
    assert_eq!(*h.app.world().get::<Position>(unit).unwrap(), position);
}

#[test]
#[ignore = "RED: failed native readiness must preserve an existing cast and movement intent"]
fn follower_kit_interrupt_out_of_range_preserves_cast_and_navigation_intent() {
    let (mut h, unit) = level32(209059);
    let candidates = two_casting_enemies(&mut h, unit);
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 133, Some(candidates[1]), false)
        .unwrap();
    let position = *h.app.world().get::<Position>(unit).unwrap();
    let intent = read_movement_intent(&h, unit);
    assert!(!try_candidates(&mut h, unit, &candidates[..1]));
    let cast = h
        .app
        .world()
        .get::<CastState>(unit)
        .expect("unready replacement preserves cast");
    assert_eq!((cast.spell_id, cast.target), (133, candidates[1].to_bits()));
    assert_eq!(read_movement_intent(&h, unit), intent);
    assert_eq!(*h.app.world().get::<Position>(unit).unwrap(), position);
}

#[test]
#[ignore = "RED: populated kit must never substitute a below-level interrupt"]
fn follower_kit_interrupt_respects_source_level_gate() {
    let (mut h, unit) = at_level(209059, 1, |_| {});
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    assert!(!try_candidates(&mut h, unit, &[enemy]));
    h.advance(3);
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 2139)
    );
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

#[test]
#[ignore = "RED: picker must retain ordinary Skull Bash parent and native-only charge motion"]
fn follower_kit_interrupt_skull_bash_motion_is_native_spell_execution_only() {
    let (mut h, unit) = level32(232800);
    prepare_native_charge_target(&mut h);
    let enemy = h.npc;
    utility_tests::prepare_enemy_spec(&mut h, enemy, 63);
    crate::spell_cast::creature_cast(h.app.world_mut(), enemy, 133, Some(unit), false).unwrap();
    let before = *h.app.world().get::<Position>(unit).unwrap();
    let target = *h.app.world().get::<Position>(enemy).unwrap();
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), enemy)
        .expect("native model reach, not a guessed default");
    let intent = read_movement_intent(&h, unit);
    assert!(try_candidates(&mut h, unit, &[enemy]));
    assert_eq!(
        read_movement_intent(&h, unit),
        intent,
        "picker never issues pursuit/retarget/follow replacement"
    );
    assert_eq!(
        *h.app.world().get::<Position>(unit).unwrap(),
        before,
        "picker does not teleport or move directly"
    );
    h.advance(20);
    let after = *h.app.world().get::<Position>(unit).unwrap();
    assert_ne!(
        after, before,
        "normal sourced charge must progress through spell execution"
    );
    let distance =
        Vec3::new(after.x, after.y, after.z).distance(Vec3::new(target.x, target.y, target.z));
    assert!(
        (distance - reach).abs() < 0.001,
        "native charge stops at the target's source reach"
    );
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.caster == unit.to_bits() && go.spell_id == 106839)
            .count(),
        1
    );
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|event| event.kind == CombatLogKind::CastSuccess
                && event.source == Some(unit.to_bits())
                && event.spell_id == Some(221514))
            .count(),
        1
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.source == Some(unit.to_bits())
                && event.spell_id == Some(93985))
    );
    assert!(
        matches!(
            crate::spell_cast::creature_cast(h.app.world_mut(), unit, 106839, Some(enemy), false),
            Err(crate::spell_cast::CastFailure::OnCooldown)
        ),
        "ordinary parent cooldown must be retained"
    );
}
