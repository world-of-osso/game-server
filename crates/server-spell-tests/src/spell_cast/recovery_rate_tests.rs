//! TC a352b1fa SpellHistory.cpp432-455/608-628: native286 scales
//! matching cooldowns and their remaining time when the aura enters/leaves.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason, apply_aura, remove_auras};
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const TRUESHOT: u32 = 288_613;
const RAPID_FIRE: u32 = 257_044;
const DEADLY_PURSUIT: u32 = 1_259_614;
const SPRINT: u32 = 2983;
const HOT_HAND: u32 = 201_900;
const HOT_HAND_PROC: u32 = 215_785;
const LAVA_LASH: u32 = 60_103;

fn fixture() -> Harness {
    harness(Vec::new(), &[TRUESHOT, RAPID_FIRE, 257_045, 263_585], 3).unwrap()
}

fn apply_trueshot(h: &mut Harness) {
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: TRUESHOT,
            // Only native286 isolates recovery; other Trueshot mechanics have
            // independent source/proof obligations.
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn cast_rapid_fire(h: &mut Harness) {
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|r| r.spell_id == Some(257_045) && r.kind == CombatLogKind::Damage)
    );
}

fn apply_deadly_pursuit(h: &mut Harness) {
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: DEADLY_PURSUIT,
            // Native286 only; charge recovery is a separate effect/contract.
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn class_completeness_native_recovery_rate_scales_native_category_only_cooldown() {
    let mut h = harness(Vec::new(), &[DEADLY_PURSUIT, SPRINT], 4).unwrap();
    apply_deadly_pursuit(&mut h);
    h.cast_and_deliver(SPRINT, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let cd = h
        .inbox
        .cooldowns
        .iter()
        .find(|r| r.spell_id == SPRINT && !r.is_gcd)
        .unwrap();
    // Native category44 only, 120000×100/(100+200)=40000ms.
    assert_eq!(cd.duration_ms, 40_000);
    assert_eq!(cd.category, 44);
}

#[test]
fn class_completeness_native_recovery_rate_category_owner_allows_real_accelerated_recast() {
    let mut h = harness(Vec::new(), &[DEADLY_PURSUIT, SPRINT], 4).unwrap();
    h.cast_and_deliver(SPRINT, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(2350);
    h.inbox.cooldowns.clear();
    apply_deadly_pursuit(&mut h);
    h.advance(3);
    let update = h
        .inbox
        .cooldowns
        .iter()
        .find(|r| r.spell_id == SPRINT && !r.is_gcd)
        .unwrap();
    assert_eq!(update.category, 44);
    assert!((780..=860).contains(&update.remaining_ms), "{update:?}");
    h.advance(5);
    h.cast_and_deliver(SPRINT, Some(h.caster));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(8);
    h.cast_and_deliver(SPRINT, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_recovery_rate_refresh_does_not_apply_the_same_factor_twice() {
    let mut h = fixture();
    apply_trueshot(&mut h);
    cast_rapid_fire(&mut h);
    apply_trueshot(&mut h);
    h.advance(95);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(65);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_recovery_rate_expiry_restores_remaining_time() {
    let mut h = fixture();
    apply_trueshot(&mut h);
    cast_rapid_fire(&mut h);
    h.advance(160);
    cast_rapid_fire(&mut h);
    // Native Trueshot expires at15s while the second shortened timer runs.
    h.advance(165);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(70);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_recovery_rate_buff_on_another_player_does_not_change_caster_timer() {
    let mut h = fixture();
    cast_rapid_fire(&mut h);
    let other = spawn_player(&mut h.app, 3, 2.0);
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: other,
            spell_id: TRUESHOT,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(230);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(45);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_recovery_rate_class_mask_excludes_trueshots_own_cooldown() {
    let mut h = fixture();
    h.cast_and_deliver(TRUESHOT, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let cd = h
        .inbox
        .cooldowns
        .iter()
        .find(|r| r.spell_id == TRUESHOT && !r.is_gcd)
        .unwrap();
    assert_eq!(cd.duration_ms, 120_000);
}

fn apply_hot_hand_proc(h: &mut Harness) {
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: HOT_HAND_PROC,
            effect_mask: 2,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn class_completeness_native_recovery_rate_native_hot_hand_modifier_refreshes_live_amount() {
    let mut h = harness(Vec::new(), &[HOT_HAND, HOT_HAND_PROC, LAVA_LASH], 7).unwrap();
    let unit = h.caster;
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            HOT_HAND_PROC,
            LAVA_LASH,
        ]));
    h.advance(3);
    h.cast_and_deliver(LAVA_LASH, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(235);
    apply_hot_hand_proc(&mut h);
    // Source label1935 PointsIndex1 adds34 to native215785's zero rate.
    // Controlled native refresh isolates amount changes, not proc availability.
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            HOT_HAND,
            HOT_HAND_PROC,
            LAVA_LASH,
        ]));
    h.advance(3);
    apply_hot_hand_proc(&mut h);
    h.advance(70);
    h.cast_and_deliver(LAVA_LASH, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(30);
    h.cast_and_deliver(LAVA_LASH, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_recovery_rate_trueshot_starts_real_ten_second_rapid_fire_cooldown() {
    let mut h = fixture();
    apply_trueshot(&mut h);
    cast_rapid_fire(&mut h);
    let cd = h
        .inbox
        .cooldowns
        .iter()
        .find(|r| r.spell_id == RAPID_FIRE && !r.is_gcd)
        .unwrap();
    // Native60% faster means16000×100/160=10000ms, not40% of16000.
    assert_eq!(cd.duration_ms, 10_000);
}

#[test]
fn class_completeness_native_recovery_rate_application_accelerates_an_existing_real_lockout() {
    let mut h = fixture();
    cast_rapid_fire(&mut h);
    // The channel has ended; accelerate only the remaining timer, not a
    // fresh full16s. Recast stays blocked6s later and succeeds9.5s later.
    h.inbox.cooldowns.clear();
    apply_trueshot(&mut h);
    h.advance(3);
    let update = h
        .inbox
        .cooldowns
        .iter()
        .find(|r| r.spell_id == RAPID_FIRE && !r.is_gcd)
        .unwrap();
    assert_eq!(update.category, 0);
    assert!((8200..=8500).contains(&update.remaining_ms), "{update:?}");
    h.advance(120);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(70);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_recovery_rate_removal_slows_only_remaining_lockout() {
    let mut h = fixture();
    apply_trueshot(&mut h);
    cast_rapid_fire(&mut h);
    let unit = h.caster;
    remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == TRUESHOT
    });
    // Undo the rate on the remaining timer only. It stays blocked10.5s
    // later and succeeds12.25s later; restarting16s would still reject.
    h.advance(210);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(35);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}
