//! Native153 IsWithinMeleeRangeAt, TC a352b1fa Unit.cpp691–709.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Npc, Position};

const EXTENDED_REACH: u32 = 326486;
const SHORT_REACH: u32 = 225319;

fn range_harness(distance: f32) -> Harness {
    let mut h = harness(
        Vec::new(),
        &[EXTENDED_REACH, SHORT_REACH, 338449, 768, 197524, 12294],
        11,
    )
    .unwrap();
    // A plain non-creature health target has no dodge/parry/block table.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .remove::<Npc>()
        .insert(Position {
            x: distance,
            y: 0.0,
            z: 0.0,
        });
    h
}

fn apply_range(h: &mut Harness, target: Entity, id: u32, index: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target,
            spell_id: id,
            effect_mask: 1 << index,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn swing_once(h: &mut Harness) -> f32 {
    use bevy::ecs::system::RunSystemOnce;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::combat::AutoAttack::new(h.npc));
    let mut mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .cloned()
        .unwrap_or_default();
    mods.melee_hit_chance_pct = 100.0;
    h.app.world_mut().entity_mut(h.caster).insert(mods);
    let before = h.health(h.npc);
    h.app
        .world_mut()
        .run_system_once(crate::combat::auto_attack_system)
        .unwrap();
    before - h.health(h.npc)
}

#[test]
fn class_completeness_native_autoattack_range_extends_actual_swing_to_ten_yards() {
    let mut h = range_harness(10.0);
    let caster = h.caster;
    apply_range(&mut h, caster, EXTENDED_REACH, 3);
    assert!(
        swing_once(&mut h) > 0.0,
        "native +5yd aura must permit real damage at 10yd"
    );
    let log = h.app.world().resource::<crate::combat::CombatLog>();
    assert!(
        log.range(0.0, f64::MAX)
            .iter()
            .any(|entry| entry.event.spell_id == 0
                && entry.event.attacker == caster.to_bits()
                && entry.event.amount > 0.0)
    );
}

#[test]
fn class_completeness_native_autoattack_range_zero_and_receiver_only_do_not_extend_swings() {
    for id in [768, 197524, EXTENDED_REACH] {
        let mut h = range_harness(6.0);
        let (target, index) = match id {
            768 => (h.caster, 8),
            197524 => (h.caster, 1),
            _ => (h.npc, 3),
        };
        apply_range(&mut h, target, id, index);
        assert_eq!(swing_once(&mut h), 0.0, "zero or receiver-only {id}");
    }
}

#[test]
fn class_completeness_native_autoattack_range_adds_independent_sources_at_inclusive_boundary() {
    for (distance, admitted) in [(20.0, true), (20.001, false)] {
        let mut h = range_harness(distance);
        let caster = h.caster;
        apply_range(&mut h, caster, EXTENDED_REACH, 3); // native +5
        apply_range(&mut h, caster, 338449, 0); // native +10
        assert_eq!(swing_once(&mut h) > 0.0, admitted, "distance {distance}");
    }
}

#[test]
fn class_completeness_native_autoattack_range_removal_restores_nominal_reach() {
    let mut h = range_harness(10.0);
    let caster = h.caster;
    apply_range(&mut h, caster, EXTENDED_REACH, 3);
    assert!(swing_once(&mut h) > 0.0);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == EXTENDED_REACH,
    );
    assert_eq!(swing_once(&mut h), 0.0);
}

#[test]
fn class_completeness_native_autoattack_range_expiry_restores_shortened_reach() {
    let mut h = range_harness(3.0);
    let caster = h.caster;
    apply_range(&mut h, caster, SHORT_REACH, 2);
    assert_eq!(swing_once(&mut h), 0.0);
    h.app
        .world_mut()
        .entity_mut(caster)
        .remove::<crate::combat::AutoAttack>();
    let duration = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SHORT_REACH)
        .unwrap()
        .duration_ms;
    assert_eq!(duration, 8000);
    h.advance(duration as usize / 50 + 2);
    assert!(!crate::auras::has_aura(h.app.world(), caster, SHORT_REACH));
    assert!(swing_once(&mut h) > 0.0);
}

#[test]
fn class_completeness_native_autoattack_range_does_not_extend_weapon_spell_range() {
    let mut h = range_harness(10.0);
    let caster = h.caster;
    apply_range(&mut h, caster, EXTENDED_REACH, 3);
    assert!(swing_once(&mut h) > 0.0);
    h.app
        .world_mut()
        .entity_mut(caster)
        .remove::<crate::combat::AutoAttack>();
    h.cast_and_deliver(12294, Some(h.npc)); // native Mortal Strike melee spell
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OutOfRange
    );
    assert!(h.inbox.go.is_empty());
}

#[test]
fn class_completeness_native_autoattack_range_refresh_replaces_the_live_cast_amount() {
    let mut h = range_harness(15.0);
    let caster = h.caster;
    apply_range(&mut h, caster, EXTENDED_REACH, 3);
    assert_eq!(swing_once(&mut h), 0.0);
    // Supported per-cast amount snapshot API, not an authored talent binding.
    let mut spell = world_db()
        .unwrap()
        .store
        .get(EXTENDED_REACH)
        .unwrap()
        .clone();
    spell
        .effects
        .iter_mut()
        .find(|effect| effect.index == 3)
        .unwrap()
        .base_points = 10.0;
    crate::auras::apply_aura_with_spell(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: EXTENDED_REACH,
            effect_mask: 1 << 3,
            source: crate::auras::AuraSource::Cast,
        },
        &spell,
    );
    assert!(swing_once(&mut h) > 0.0);
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 15.001;
    assert_eq!(
        swing_once(&mut h),
        0.0,
        "refresh must replace, not add old5+new10"
    );
}

#[test]
fn class_completeness_native_autoattack_range_extends_off_hand_swings() {
    use bevy::ecs::system::RunSystemOnce;
    let mut h = range_harness(10.0);
    let caster = h.caster;
    let weapon = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .main_hand;
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .off_hand = Some(weapon);
    apply_range(&mut h, caster, EXTENDED_REACH, 3);
    assert!(swing_once(&mut h) > 0.0);
    let before = h.health(h.npc);
    let mut attack = h
        .app
        .world_mut()
        .get_mut::<crate::combat::AutoAttack>(caster)
        .unwrap();
    attack.timer = 99.0;
    attack.off_hand_timer = 0.0;
    drop(attack);
    h.app
        .world_mut()
        .run_system_once(crate::combat::auto_attack_system)
        .unwrap();
    assert!(
        h.health(h.npc) < before,
        "ready off-hand must deal actual damage at 10yd"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(caster)
            .unwrap()
            .off_hand_timer
            > 0.0
    );
}

#[test]
fn class_completeness_native_autoattack_range_negative_restricts_actual_swing() {
    let mut h = range_harness(3.0);
    let caster = h.caster;
    apply_range(&mut h, caster, SHORT_REACH, 2);
    assert_eq!(
        swing_once(&mut h),
        0.0,
        "native -3yd aura restricts nominal5yd range to2yd"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(caster)
            .unwrap()
            .timer
            <= 0.0
    );
}
