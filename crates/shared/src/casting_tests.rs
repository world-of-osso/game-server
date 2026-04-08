use super::*;

#[test]
fn normal_cast_in_progress() {
    let mut cast = CastState::normal(133, 1, 2.0, true);
    assert_eq!(cast.tick(0.5), CastTickResult::InProgress);
    assert!((cast.progress() - 0.25).abs() < 0.01);
}

#[test]
fn normal_cast_completes() {
    let mut cast = CastState::normal(133, 1, 2.0, true);
    cast.tick(1.5);
    assert_eq!(cast.tick(0.6), CastTickResult::Completed);
}

#[test]
fn normal_cast_instant_completes_immediately() {
    let mut cast = CastState::normal(100, 1, 0.0, false);
    assert_eq!(cast.tick(0.016), CastTickResult::Completed);
}

#[test]
fn channel_ticks_during_duration() {
    let mut cast = CastState::channel(5143, 1, 4.0, 1.0, true);
    assert_eq!(cast.tick(0.5), CastTickResult::InProgress);
    assert_eq!(cast.tick(0.6), CastTickResult::ChannelTick); // 1.1s total
}

#[test]
fn channel_multiple_ticks() {
    let mut cast = CastState::channel(5143, 1, 6.0, 2.0, true);
    assert_eq!(cast.tick(1.5), CastTickResult::InProgress);
    assert_eq!(cast.tick(0.6), CastTickResult::ChannelTick); // tick at 2.1s
    assert_eq!(cast.tick(1.5), CastTickResult::InProgress);
    assert_eq!(cast.tick(0.6), CastTickResult::ChannelTick); // tick at 4.2s
}

#[test]
fn channel_completes_after_duration() {
    let mut cast = CastState::channel(5143, 1, 3.0, 1.0, true);
    cast.tick(1.5);
    cast.tick(1.0);
    assert_eq!(cast.tick(1.0), CastTickResult::ChannelComplete);
}

#[test]
fn cast_progress_clamped() {
    let mut cast = CastState::normal(133, 1, 2.0, true);
    cast.tick(5.0);
    assert_eq!(cast.progress(), 1.0);
}

#[test]
fn cast_progress_zero_duration() {
    let cast = CastState::normal(100, 1, 0.0, false);
    assert_eq!(cast.progress(), 1.0);
}

#[test]
fn pushback_extends_normal_cast() {
    let mut cast = CastState::normal(133, 1, 2.0, true);
    cast.tick(1.0);
    assert!(cast.apply_pushback());
    assert!((cast.duration - 2.5).abs() < 0.001);
}

#[test]
fn pushback_max_two() {
    let mut cast = CastState::normal(133, 1, 2.0, true);
    assert!(cast.apply_pushback()); // 2.5s
    assert!(cast.apply_pushback()); // 3.0s
    assert!(!cast.apply_pushback()); // rejected
    assert!((cast.duration - 3.0).abs() < 0.001);
}

#[test]
fn pushback_non_interruptible_ignored() {
    let mut cast = CastState::normal(100, 1, 2.0, false);
    assert!(!cast.apply_pushback());
    assert!((cast.duration - 2.0).abs() < 0.001);
}

#[test]
fn pushback_channel_reduces_duration() {
    let mut cast = CastState::channel(5143, 1, 4.0, 1.0, true);
    cast.tick(0.5);
    assert!(cast.apply_pushback());
    // 4.0 - (4.0 * 0.25) = 3.0
    assert!((cast.duration - 3.0).abs() < 0.001);
}

#[test]
fn pushback_channel_max_two() {
    let mut cast = CastState::channel(5143, 1, 4.0, 1.0, true);
    assert!(cast.apply_pushback()); // 3.0s
    assert!(cast.apply_pushback()); // 3.0 - 0.75 = 2.25s (25% of 3.0)
    assert!(!cast.apply_pushback());
}

#[test]
fn pushback_channel_never_below_elapsed() {
    let mut cast = CastState::channel(5143, 1, 2.0, 0.5, true);
    cast.tick(1.8); // almost done
    assert!(cast.apply_pushback());
    // 2.0 - 0.5 = 1.5, but elapsed=1.8, so clamped to 1.8
    assert!(cast.duration >= cast.elapsed);
}

#[test]
fn lockout_initially_empty() {
    let lockouts = SchoolLockouts::default();
    assert!(!lockouts.is_locked(SpellSchool::Fire));
}

#[test]
fn lockout_blocks_school() {
    let mut lockouts = SchoolLockouts::default();
    lockouts.lock(SpellSchool::Fire, 5.0);
    assert!(lockouts.is_locked(SpellSchool::Fire));
    assert!(!lockouts.is_locked(SpellSchool::Frost));
}

#[test]
fn lockout_expires_after_tick() {
    let mut lockouts = SchoolLockouts::default();
    lockouts.lock(SpellSchool::Shadow, 3.0);
    lockouts.tick(2.0);
    assert!(lockouts.is_locked(SpellSchool::Shadow));
    lockouts.tick(1.5);
    assert!(!lockouts.is_locked(SpellSchool::Shadow));
}

#[test]
fn lockout_refresh_keeps_longer() {
    let mut lockouts = SchoolLockouts::default();
    lockouts.lock(SpellSchool::Fire, 3.0);
    lockouts.lock(SpellSchool::Fire, 5.0);
    lockouts.tick(3.5);
    assert!(lockouts.is_locked(SpellSchool::Fire));
}

#[test]
fn lockout_refresh_ignores_shorter() {
    let mut lockouts = SchoolLockouts::default();
    lockouts.lock(SpellSchool::Fire, 5.0);
    lockouts.lock(SpellSchool::Fire, 2.0);
    lockouts.tick(4.5);
    assert!(lockouts.is_locked(SpellSchool::Fire));
}

#[test]
fn lockout_multiple_schools() {
    let mut lockouts = SchoolLockouts::default();
    lockouts.lock(SpellSchool::Fire, 3.0);
    lockouts.lock(SpellSchool::Frost, 5.0);
    lockouts.tick(4.0);
    assert!(!lockouts.is_locked(SpellSchool::Fire));
    assert!(lockouts.is_locked(SpellSchool::Frost));
}

#[test]
fn cooldown_initially_ready() {
    let cds = SpellCooldowns::default();
    assert!(!cds.is_on_cooldown(133));
    assert_eq!(cds.remaining(133), 0.0);
}

#[test]
fn cooldown_start_and_check() {
    let mut cds = SpellCooldowns::default();
    cds.start(133, 8.0);
    assert!(cds.is_on_cooldown(133));
    assert_eq!(cds.remaining(133), 8.0);
    assert!(!cds.is_on_cooldown(200));
}

#[test]
fn cooldown_ticks_down() {
    let mut cds = SpellCooldowns::default();
    cds.start(133, 5.0);
    cds.tick(3.0);
    assert!((cds.remaining(133) - 2.0).abs() < 0.001);
    assert!(cds.is_on_cooldown(133));
}

#[test]
fn cooldown_expires() {
    let mut cds = SpellCooldowns::default();
    cds.start(133, 5.0);
    cds.tick(5.5);
    assert!(!cds.is_on_cooldown(133));
    assert_eq!(cds.remaining(133), 0.0);
}

#[test]
fn cooldown_reset_specific() {
    let mut cds = SpellCooldowns::default();
    cds.start(100, 10.0);
    cds.start(200, 15.0);
    cds.reset(100);
    assert!(!cds.is_on_cooldown(100));
    assert!(cds.is_on_cooldown(200));
}

#[test]
fn cooldown_clear_all() {
    let mut cds = SpellCooldowns::default();
    cds.start(100, 10.0);
    cds.start(200, 15.0);
    cds.clear_all();
    assert!(!cds.is_on_cooldown(100));
    assert!(!cds.is_on_cooldown(200));
}

#[test]
fn cooldown_start_zero_duration_ignored() {
    let mut cds = SpellCooldowns::default();
    cds.start(133, 0.0);
    assert!(!cds.is_on_cooldown(133));
}

#[test]
fn cooldown_restart_resets_timer() {
    let mut cds = SpellCooldowns::default();
    cds.start(133, 10.0);
    cds.tick(5.0);
    cds.start(133, 10.0);
    assert!((cds.remaining(133) - 10.0).abs() < 0.001);
}

#[test]
fn shared_cd_triggers_category_members() {
    let mut cds = SpellCooldowns::default();
    cds.register_category(100, 1);
    cds.register_category(200, 1);
    cds.start_with_category(100, 1, 60.0);
    assert!(cds.is_on_cooldown(100));
    assert!(cds.is_on_cooldown(200));
}

#[test]
fn shared_cd_independent_unaffected() {
    let mut cds = SpellCooldowns::default();
    cds.register_category(100, 1);
    cds.start(200, 8.0);
    cds.start_with_category(100, 1, 60.0);
    assert!((cds.remaining(200) - 8.0).abs() < 0.001);
}

#[test]
fn shared_cd_different_categories_independent() {
    let mut cds = SpellCooldowns::default();
    cds.register_category(100, 1);
    cds.register_category(200, 2);
    cds.start_with_category(100, 1, 60.0);
    assert!(!cds.is_on_cooldown(200));
}

#[test]
fn shared_cd_category_tick_preserves_entries() {
    let mut cds = SpellCooldowns::default();
    cds.register_category(100, 1);
    cds.register_category(200, 1);
    cds.start_with_category(100, 1, 5.0);
    cds.tick(6.0);
    assert!(!cds.is_on_cooldown(100));
    assert!(!cds.is_on_cooldown(200));
    cds.start_with_category(200, 1, 10.0);
    assert!(cds.is_on_cooldown(100));
    assert!(cds.is_on_cooldown(200));
}

#[test]
fn shared_cd_keeps_longer_existing() {
    let mut cds = SpellCooldowns::default();
    cds.register_category(100, 1);
    cds.register_category(200, 1);
    cds.start_with_category(100, 1, 60.0);
    cds.start_with_category(200, 1, 10.0);
    assert!(cds.remaining(100) >= 59.0);
}

#[test]
fn charge_starts_full() {
    let entry = ChargeEntry::new(100, 2, 15.0);
    assert_eq!(entry.charges, 2);
    assert!(entry.available());
}

#[test]
fn charge_use_depletes() {
    let mut entry = ChargeEntry::new(100, 2, 15.0);
    assert!(entry.use_charge());
    assert_eq!(entry.charges, 1);
    assert!(entry.use_charge());
    assert_eq!(entry.charges, 0);
    assert!(!entry.available());
    assert!(!entry.use_charge());
}

#[test]
fn charge_recharges_over_time() {
    let mut entry = ChargeEntry::new(100, 2, 10.0);
    entry.use_charge();
    entry.use_charge();
    assert_eq!(entry.charges, 0);

    entry.tick(10.5);
    assert_eq!(entry.charges, 1);

    entry.tick(10.0);
    assert_eq!(entry.charges, 2);
}

#[test]
fn charge_recharge_stops_at_max() {
    let mut entry = ChargeEntry::new(100, 2, 10.0);
    entry.use_charge();
    entry.tick(15.0);
    assert_eq!(entry.charges, 2);
    assert_eq!(entry.recharge_timer, 0.0);
}

#[test]
fn charge_partial_recharge_preserved() {
    let mut entry = ChargeEntry::new(100, 3, 10.0);
    entry.use_charge();
    entry.use_charge();

    entry.tick(5.0);
    assert_eq!(entry.charges, 1);
    assert!((entry.recharge_timer - 5.0).abs() < 0.001);

    entry.tick(6.0);
    assert_eq!(entry.charges, 2);
    assert!((entry.recharge_timer - 1.0).abs() < 0.001);
}

#[test]
fn spell_charges_component() {
    let mut charges = SpellCharges::default();
    charges.add(ChargeEntry::new(100, 2, 15.0));
    charges.add(ChargeEntry::new(200, 3, 10.0));

    assert!(charges.use_charge(100));
    assert_eq!(charges.get(100).unwrap().charges, 1);
    assert!(!charges.use_charge(999));
}

#[test]
fn spell_charges_tick_all() {
    let mut charges = SpellCharges::default();
    charges.add(ChargeEntry::new(100, 2, 10.0));
    charges.add(ChargeEntry::new(200, 2, 5.0));
    charges.use_charge(100);
    charges.use_charge(200);

    charges.tick(5.5);
    assert_eq!(charges.get(100).unwrap().charges, 1);
    assert_eq!(charges.get(200).unwrap().charges, 2);
}

#[test]
fn spell_batch_queue_and_drain() {
    let mut batch = SpellBatch::default();
    assert!(batch.is_empty());

    batch.queue(PendingSpellCast {
        caster: 1,
        target: 2,
        spell_id: 133,
    });
    batch.queue(PendingSpellCast {
        caster: 3,
        target: 4,
        spell_id: 116,
    });
    assert_eq!(batch.len(), 2);

    let drained = batch.drain();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].spell_id, 133);
    assert_eq!(drained[1].spell_id, 116);
    assert!(batch.is_empty());
}

#[test]
fn spell_batch_drain_clears() {
    let mut batch = SpellBatch::default();
    batch.queue(PendingSpellCast {
        caster: 1,
        target: 2,
        spell_id: 100,
    });
    batch.drain();
    batch.drain();
    assert!(batch.is_empty());
}

#[test]
fn spell_batch_preserves_order() {
    let mut batch = SpellBatch::default();
    for i in 0..5 {
        batch.queue(PendingSpellCast {
            caster: 1,
            target: 2,
            spell_id: i,
        });
    }
    let drained = batch.drain();
    let ids: Vec<u32> = drained.iter().map(|c| c.spell_id).collect();
    assert_eq!(ids, vec![0, 1, 2, 3, 4]);
}

#[test]
fn projectile_travel_time_from_distance() {
    let proj = SpellProjectile::new(133, 1, 2, 48.0, 24.0);
    assert!((proj.remaining - 2.0).abs() < 0.001);
}

#[test]
fn projectile_default_speed() {
    let proj = SpellProjectile::new(133, 1, 2, 24.0, 0.0);
    assert!((proj.remaining - 1.0).abs() < 0.001);
}

#[test]
fn projectile_tick_impacts() {
    let mut proj = SpellProjectile::new(133, 1, 2, 24.0, 24.0);
    assert!(!proj.tick(0.5));
    assert!(proj.tick(0.6));
}

#[test]
fn projectiles_collect_impacts() {
    let mut projs = SpellProjectiles::default();
    projs.launch(SpellProjectile::new(133, 1, 2, 24.0, 24.0));
    projs.launch(SpellProjectile::new(116, 1, 3, 72.0, 24.0));

    let impacts = projs.tick_and_collect_impacts(1.5);
    assert_eq!(impacts.len(), 1);
    assert_eq!(impacts[0].spell_id, 133);
    assert_eq!(projs.active.len(), 1);
}

#[test]
fn projectiles_all_impact() {
    let mut projs = SpellProjectiles::default();
    projs.launch(SpellProjectile::new(100, 1, 2, 12.0, 24.0));
    projs.launch(SpellProjectile::new(200, 1, 3, 24.0, 24.0));

    let impacts = projs.tick_and_collect_impacts(2.0);
    assert_eq!(impacts.len(), 2);
    assert!(projs.active.is_empty());
}

#[test]
fn projectiles_empty_no_impacts() {
    let mut projs = SpellProjectiles::default();
    let impacts = projs.tick_and_collect_impacts(1.0);
    assert!(impacts.is_empty());
}
