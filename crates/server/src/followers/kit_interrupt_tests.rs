//! Native provider and ordinary follower-update interrupt contract.
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
fn follower_kit_interrupt_does_not_spend_on_noninterruptible_cast() {
    let (mut h, unit) = level32(209059);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    utility_tests::protect_enemy_cast_with_native_interrupt_immunity(&mut h, enemy);
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
fn follower_kit_interrupt_uses_native_eligibility_not_pushback_flag() {
    let (mut h, unit) = level32(209059);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    h.app
        .world_mut()
        .get_mut::<CastState>(enemy)
        .unwrap()
        .interruptible = false;
    assert!(crate::spell_cast::can_interrupt_cast(
        h.app.world(),
        unit,
        enemy
    ));
    assert!(try_candidates(&mut h, unit, &[enemy]));
    h.advance(3);
    assert!(h.app.world().get::<CastState>(enemy).is_none());
}

#[test]
fn follower_kit_interrupt_runtime_garrick32_uses_native_kick() {
    let (mut h, unit) = level32(209057);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    h.app
        .world_mut()
        .get_mut::<CastState>(enemy)
        .unwrap()
        .interruptible = false;
    super::update_ai(h.app.world_mut());
    h.advance(3);
    assert!(h.app.world().get::<CastState>(enemy).is_none());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.source == Some(unit.to_bits())
                && event.spell_id == Some(96231))
    );
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 31935)
    );
}

#[test]
fn follower_kit_interrupt_runtime_garrick26_heal_precedes_shield() {
    let (mut h, unit) = at_level(209057, 26, |_| {});
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    let mut health = h
        .app
        .world_mut()
        .get_mut::<shared::components::Health>(unit)
        .unwrap();
    health.current = health.max * 0.15;
    super::update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 633)
    );
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 31935)
    );
    assert!(h.app.world().get::<CastState>(enemy).is_some());
}

#[test]
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

fn install_native_catalog(h: &mut Harness) {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::catalog::load_catalog(&db).unwrap());
}

#[test]
fn follower_kit_interrupt_absent_rows_preserve_legacy_kick() {
    let (mut h, unit) = level32(209059);
    h.app
        .world_mut()
        .resource_mut::<super::FollowerCatalog>()
        .kits
        .clear();
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    assert!(try_candidates(&mut h, unit, &[enemy]));
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.source == Some(unit.to_bits())
                && event.spell_id == Some(2139))
    );
}

#[test]
fn follower_kit_interrupt_actual_felguard_keeps_owner_cast() {
    let (mut fixture, pet) = super::pet_tests::felguard_at_level(17);
    install_native_catalog(&mut fixture.h);
    utility_tests::start_enemy_fireball(&mut fixture.h, pet);
    let enemy = fixture.h.npc;
    let unit = fixture.unit;
    crate::spell_cast::creature_cast(fixture.h.app.world_mut(), unit, 686, Some(enemy), false)
        .unwrap();
    assert!(try_candidates(&mut fixture.h, unit, &[enemy]));
    fixture.h.advance(8);
    assert_eq!(
        fixture
            .h
            .app
            .world()
            .get::<CastState>(unit)
            .unwrap()
            .spell_id,
        686
    );
    assert!(
        fixture
            .h
            .inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.source == Some(pet.to_bits())
                && event.spell_id == Some(347008))
    );
}

#[test]
fn follower_kit_interrupt_unlearned_pet_spell_is_not_granted() {
    let (mut fixture, pet) = super::pet_tests::felguard_at_level(16);
    install_native_catalog(&mut fixture.h);
    assert!(
        fixture
            .h
            .app
            .world()
            .get::<crate::pet_control::PetCharm>(pet)
            .unwrap()
            .spell_slot(89766)
            .is_none()
    );
    utility_tests::start_enemy_fireball(&mut fixture.h, pet);
    let enemy = fixture.h.npc;
    assert!(!try_candidates(&mut fixture.h, fixture.unit, &[enemy]));
    fixture.h.advance(8);
    assert!(fixture.h.app.world().get::<CastState>(enemy).is_some());
    assert!(
        !fixture
            .h
            .inbox
            .go
            .iter()
            .any(|go| go.caster == pet.to_bits() && go.spell_id == 89766)
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
    // Full-mask children intentionally emit no CastSuccess event. Compare the
    // actual motion with an ordinary parent cast, not an invented wire message.
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
    assert_eq!(
        *native.app.world().get::<Position>(native_unit).unwrap(),
        after,
        "picker motion must equal ordinary sourced spell execution"
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
