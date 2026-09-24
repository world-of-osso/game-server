//! Aura runtime tests with real spells from data/world.db (skipped when absent).

use shared::casting::CastState;
use shared::components::{
    AuraView, Health, Mana, Player, Position, UnitAuras, UnitFactionTemplate,
};
use shared::protocol::{CancelAura, CombatChannel, CombatLogEvent, CombatLogKind};

use super::*;
use crate::networking::CombatTarget;
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const SHADOW_WORD_PAIN: u32 = 589;
const RENEW: u32 = 139;
const PURIFY: u32 = 527;
const FLASH_HEAL: u32 = 2061;
const SELFLESS_HEALER: u32 = 114250;
/// Warrior passive: PROC_TRIGGER_SPELL 42 on DEAL_MELEE_SWING (mask 0x4),
/// 100% chance, triggers 236282 (AP-scaled damage).
const DEVASTATOR: u32 = 236279;
const DEVASTATOR_DAMAGE: u32 = 236282;
/// Priest passive: ADD_PCT_MODIFIER 108, ChangeCastTime (10) -5% on
/// SpellClassSet 6 mask_0 0xA80, which matches Flash Heal (mask_0 0x800).
const UNWAVERING_WILL: u32 = 373456;
/// Priest passive: ADD_PCT_MODIFIER 108, PowerCost0 (14) -50% on
/// SpellClassSet 6 mask_1 0x81, which matches Purify (mask_1 0x1).
const MENTAL_AGILITY: u32 = 341167;
/// MOD_INCREASE_SPEED 31 +70% on self.
const SPRINT: u32 = 2983;
/// MOD_DECREASE_SPEED 33 -50%.
const HAMSTRING: u32 = 1715;
/// MOD_ROOT_2 455.
const FROST_NOVA: u32 = 122;

fn apply(app: &mut App, caster: Entity, target: Entity, spell_id: u32) {
    let spell = app
        .world()
        .resource::<SpellStore>()
        .get(spell_id)
        .cloned()
        .unwrap();
    apply_aura(
        app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: aura_effect_mask(&spell),
            source: AuraSource::Cast,
        },
    );
}

fn auras_of(h: &Harness, unit: Entity, spell_id: u32) -> Vec<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(unit)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| aura.spell_id == spell_id)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn view_of(h: &Harness, unit: Entity, spell_id: u32) -> Option<AuraView> {
    h.app.world().get::<UnitAuras>(unit).and_then(|view| {
        view.auras
            .iter()
            .find(|aura| aura.spell_id == spell_id)
            .cloned()
    })
}

fn log_of(h: &Harness, kind: CombatLogKind, spell_id: u32) -> Vec<CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| event.kind == kind && event.spell_id == Some(spell_id))
        .cloned()
        .collect()
}

fn frames_for(ms: u32) -> usize {
    (ms / 50) as usize
}

#[test]
fn shadow_word_pain_ticks_periodic_damage_then_expires() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_WORD_PAIN], 5) else {
        return;
    };
    h.cast_and_deliver(SHADOW_WORD_PAIN, Some(h.npc));
    // Effect 0: 0.168 × SP 100 = 16.8 → 17 direct. Effect 1: PERIODIC_DAMAGE
    // 0.1246 × 100 = 12.46 → 12 every 2 s for 16 s (duration index 387).
    assert_eq!(h.health(h.npc), 5000.0 - 17.0);
    let aura = auras_of(&h, h.npc, SHADOW_WORD_PAIN);
    assert_eq!(aura.len(), 1);
    assert!(aura[0].harmful);
    assert_eq!(aura[0].duration_ms, 16_000);
    h.advance(frames_for(16_000) + 4);
    let ticks: Vec<_> = log_of(&h, CombatLogKind::Damage, SHADOW_WORD_PAIN)
        .into_iter()
        .filter(|event| event.periodic)
        .collect();
    assert_eq!(ticks.len(), 8);
    assert!(ticks.iter().all(|tick| tick.amount == 12));
    assert_eq!(h.health(h.npc), 5000.0 - 17.0 - 8.0 * 12.0);
    assert!(auras_of(&h, h.npc, SHADOW_WORD_PAIN).is_empty());
    assert!(view_of(&h, h.npc, SHADOW_WORD_PAIN).is_none());
    assert_eq!(
        log_of(&h, CombatLogKind::AuraApplied, SHADOW_WORD_PAIN).len(),
        1
    );
    assert_eq!(
        log_of(&h, CombatLogKind::AuraRemoved, SHADOW_WORD_PAIN).len(),
        1
    );
}

#[test]
fn renew_heals_every_tick() {
    let Some(mut h) = harness(Vec::new(), &[RENEW], 5) else {
        return;
    };
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 500.0;
    h.cast_and_deliver(RENEW, None);
    // PERIODIC_HEAL 0.60928 × SP 100 = 60.9 → 61 every 3 s for 15 s.
    h.advance(frames_for(15_000) + 4);
    let ticks: Vec<_> = log_of(&h, CombatLogKind::Heal, RENEW)
        .into_iter()
        .filter(|event| event.periodic)
        .collect();
    assert_eq!(ticks.len(), 5);
    assert!(ticks.iter().all(|tick| tick.amount == 61));
    assert_eq!(h.health(h.caster), 500.0 + 5.0 * 61.0);
}

#[test]
fn stacking_aura_increments_to_max_stacks() {
    let Some(mut h) = harness(Vec::new(), &[SELFLESS_HEALER], 2) else {
        return;
    };
    for _ in 0..6 {
        apply(&mut h.app, h.caster, h.caster, SELFLESS_HEALER);
    }
    let aura = auras_of(&h, h.caster, SELFLESS_HEALER);
    assert_eq!(aura.len(), 1);
    assert_eq!((aura[0].stacks, aura[0].max_stacks), (4, 4));
    h.advance(1);
    assert_eq!(view_of(&h, h.caster, SELFLESS_HEALER).unwrap().stacks, 4);
}

#[test]
fn same_caster_refresh_resets_duration_and_other_casters_coexist() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_WORD_PAIN], 5) else {
        return;
    };
    apply(&mut h.app, h.caster, h.npc, SHADOW_WORD_PAIN);
    let first_expiry = auras_of(&h, h.npc, SHADOW_WORD_PAIN)[0].expires_at.unwrap();
    h.advance(40);
    apply(&mut h.app, h.caster, h.npc, SHADOW_WORD_PAIN);
    let aura = auras_of(&h, h.npc, SHADOW_WORD_PAIN);
    assert_eq!(aura.len(), 1);
    let refreshed = aura[0].expires_at.unwrap();
    assert!(
        (refreshed - first_expiry - 2.0).abs() < 1e-6,
        "{refreshed} vs {first_expiry}"
    );
    h.advance(1);
    assert_eq!(
        log_of(&h, CombatLogKind::AuraRefreshed, SHADOW_WORD_PAIN).len(),
        1
    );

    let other = h
        .app
        .world_mut()
        .spawn((
            Player {
                name: "Other".into(),
                race: 1,
                class: 5,
                appearance: default(),
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    apply(&mut h.app, other, h.npc, SHADOW_WORD_PAIN);
    assert_eq!(auras_of(&h, h.npc, SHADOW_WORD_PAIN).len(), 2);
}

/// Monster template 14 (EnemyGroup includes players): the NPC regards the
/// caster as hostile, so its auras on the caster are harmful. The harness
/// kobold (25) is only hostile from the player side.
fn make_npc_hostile_to_players(h: &mut Harness) {
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(UnitFactionTemplate(14));
}

fn cancel(h: &mut Harness, spell_id: u32) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<CancelAura>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(CancelAura { spell_id });
    h.advance(3);
}

#[test]
fn cancel_aura_removes_own_buff_but_not_a_debuff() {
    let Some(mut h) = harness(Vec::new(), &[RENEW, SHADOW_WORD_PAIN], 5) else {
        return;
    };
    make_npc_hostile_to_players(&mut h);
    apply(&mut h.app, h.caster, h.caster, RENEW);
    apply(&mut h.app, h.npc, h.caster, SHADOW_WORD_PAIN);
    cancel(&mut h, SHADOW_WORD_PAIN);
    cancel(&mut h, RENEW);
    assert!(auras_of(&h, h.caster, RENEW).is_empty());
    assert_eq!(auras_of(&h, h.caster, SHADOW_WORD_PAIN).len(), 1);
    assert!(view_of(&h, h.caster, RENEW).is_none());
    assert!(view_of(&h, h.caster, SHADOW_WORD_PAIN).unwrap().harmful);
}

#[test]
fn purify_dispels_a_magic_debuff() {
    let Some(mut h) = harness(Vec::new(), &[PURIFY, SHADOW_WORD_PAIN], 5) else {
        return;
    };
    make_npc_hostile_to_players(&mut h);
    apply(&mut h.app, h.npc, h.caster, SHADOW_WORD_PAIN);
    assert_eq!(auras_of(&h, h.caster, SHADOW_WORD_PAIN)[0].dispel_type, 1);
    h.cast_and_deliver(PURIFY, None);
    assert!(auras_of(&h, h.caster, SHADOW_WORD_PAIN).is_empty());
    let dispel = log_of(&h, CombatLogKind::Dispel, SHADOW_WORD_PAIN);
    assert_eq!(dispel.len(), 1);
    assert_eq!(dispel[0].source, Some(h.caster.to_bits()));
}

#[test]
fn passive_proc_trigger_fires_on_melee_swing() {
    let Some(mut h) = harness(Vec::new(), &[DEVASTATOR, DEVASTATOR_DAMAGE], 1) else {
        return;
    };
    h.advance(1);
    let passive = auras_of(&h, h.caster, DEVASTATOR);
    assert_eq!(passive.len(), 1, "known passive auto-applied");
    assert!(passive[0].passive && passive[0].hidden && passive[0].expires_at.is_none());
    let npc = h.npc;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatTarget(Some(npc)));
    h.advance(3);
    let procs: Vec<_> = log_of(&h, CombatLogKind::Damage, DEVASTATOR_DAMAGE);
    assert_eq!(procs.len(), 1, "one swing, one proc");
    assert_eq!(procs[0].source, Some(h.caster.to_bits()));
    assert_eq!(procs[0].target, Some(npc.to_bits()));
    // 236282 effect 0: AP coefficient 1.314 × AP 100.
    assert_eq!(procs[0].amount, 131);
}

#[test]
fn pct_modifier_talent_shortens_matched_cast_time() {
    let Some(mut h) = harness(Vec::new(), &[UNWAVERING_WILL, FLASH_HEAL], 5) else {
        return;
    };
    h.advance(1);
    h.cast_and_deliver(FLASH_HEAL, None);
    let cast = h.app.world().get::<CastState>(h.caster).unwrap();
    // Flash Heal base 1500 ms × (1 - 5%).
    assert!((cast.duration - 1.425).abs() < 1e-6, "{}", cast.duration);
}

#[test]
fn pct_modifier_talent_halves_matched_cost() {
    let Some(mut h) = harness(Vec::new(), &[MENTAL_AGILITY, PURIFY], 5) else {
        return;
    };
    h.advance(1);
    let purify = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(PURIFY)
        .cloned()
        .unwrap();
    let costs = crate::spell_cast::spell_costs(h.app.world(), h.caster, &purify).unwrap();
    // Purify: 1.3% of max mana 10000 = 130, halved by Mental Agility.
    assert_eq!(costs.len(), 1);
    assert!((costs[0].1 - 65.0).abs() < 1e-3, "{costs:?}");
    let before = h.app.world().get::<Mana>(h.caster).unwrap().current;
    h.cast_and_deliver(PURIFY, None);
    let after = h.app.world().get::<Mana>(h.caster).unwrap().current;
    assert!(
        before - after > 50.0,
        "cast spent the reduced cost: {before} → {after}"
    );
}

#[test]
fn unit_auras_report_remaining_time_at_last_change() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_WORD_PAIN, RENEW], 5) else {
        return;
    };
    apply(&mut h.app, h.caster, h.npc, SHADOW_WORD_PAIN);
    h.advance(1);
    let first = view_of(&h, h.npc, SHADOW_WORD_PAIN).unwrap();
    assert_eq!(first.duration_ms, 16_000);
    assert!(first.remaining_ms > 15_900 && first.remaining_ms <= 16_000);
    h.advance(20);
    assert_eq!(
        view_of(&h, h.npc, SHADOW_WORD_PAIN).unwrap(),
        first,
        "no resend without a change (ticks do not change the view)"
    );
    apply(&mut h.app, h.npc, h.npc, RENEW);
    h.advance(1);
    let later = view_of(&h, h.npc, SHADOW_WORD_PAIN).unwrap();
    assert!(
        (i64::from(first.remaining_ms) - i64::from(later.remaining_ms) - 1050).abs() <= 50,
        "{} → {}",
        first.remaining_ms,
        later.remaining_ms
    );
    assert_eq!(
        later.flags & AuraView::FLAG_FROM_PLAYER,
        AuraView::FLAG_FROM_PLAYER
    );
}

#[test]
fn unlearned_passive_is_removed() {
    let Some(mut h) = harness(Vec::new(), &[UNWAVERING_WILL], 5) else {
        return;
    };
    h.advance(1);
    assert_eq!(auras_of(&h, h.caster, UNWAVERING_WILL).len(), 1);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    // Queued in Update, synced in the next frame's FixedUpdate.
    h.advance(2);
    assert!(auras_of(&h, h.caster, UNWAVERING_WILL).is_empty());
}

#[test]
fn movement_speed_takes_strongest_increase_and_slow_and_roots_stop() {
    let Some(mut h) = harness(Vec::new(), &[SPRINT, HAMSTRING, FROST_NOVA], 4) else {
        return;
    };
    let speed = |h: &Harness| {
        h.app
            .world()
            .get::<AuraStatMods>(h.caster)
            .unwrap()
            .movement_speed
    };
    apply(&mut h.app, h.caster, h.caster, SPRINT);
    assert!((speed(&h) - 1.7).abs() < 1e-6);
    apply(&mut h.app, h.npc, h.caster, HAMSTRING);
    assert!((speed(&h) - 1.7 * 0.5).abs() < 1e-6);
    apply(&mut h.app, h.npc, h.caster, FROST_NOVA);
    assert_eq!(speed(&h), 0.0);
}

#[test]
fn stored_spells_carry_labels() {
    let Some(data) = world_db() else {
        return;
    };
    // SpellLabel rows 75/76: labels 16 and 17 on spell 44425.
    let labels = &data.store.get(44425).unwrap().labels;
    assert!(labels.contains(&16) && labels.contains(&17), "{labels:?}");
}

/// Shadow Priest spec passive: HealingAndDamage mods on Mind Blast from
/// class-mask 108 effects (idx 0: -7% mask_2 0x100, idx 11: +300% mask_0
/// 0x2000) and label 218 effect idx 12 (+137%, label 4040).
const SHADOW_PRIEST: u32 = 137033;
const MIND_BLAST: u32 = 8092;

#[test]
fn class_mask_and_label_modifiers_scale_damage() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_PRIEST, MIND_BLAST], 5) else {
        return;
    };
    h.advance(1);
    h.cast_and_deliver(MIND_BLAST, Some(h.npc));
    h.advance(40);
    let hits = log_of(&h, CombatLogKind::Damage, MIND_BLAST);
    assert_eq!(hits.len(), 1, "{:?}", h.inbox.failed);
    // 0.78336 × SP 100 × 0.93 × 4.00 × 2.37 = 690.6.
    assert_eq!(hits[0].amount, 691);
}

/// Warrior talent: MOD_MAX_CHARGES 411 +1 and CHARGE_RECOVERY_MOD 453
/// -3000 ms on charge category 1386 (Charge 100: 1 charge, 20 s).
const DOUBLE_TIME: u32 = 103827;
const CHARGE: u32 = 100;

#[test]
fn charge_auras_add_a_charge_and_shorten_recovery() {
    let Some(mut h) = harness(Vec::new(), &[DOUBLE_TIME, CHARGE], 1) else {
        return;
    };
    h.advance(1);
    h.cast_and_deliver(CHARGE, Some(h.npc));
    h.advance(2);
    let charges = std::mem::take(&mut h.inbox.charges);
    assert_eq!(
        (charges[0].current, charges[0].max, charges[0].recharge_ms),
        (1, 2, 17_000)
    );
}

/// MOD_STUN 12.
const HAMMER_OF_JUSTICE: u32 = 853;
/// MOD_SILENCE 27.
const SILENCE: u32 = 15487;

#[test]
fn stun_and_silence_block_casting() {
    use shared::spell_data::CastFailReason;
    let Some(mut h) = harness(Vec::new(), &[FLASH_HEAL, HAMMER_OF_JUSTICE, SILENCE], 5) else {
        return;
    };
    apply(&mut h.app, h.npc, h.caster, SILENCE);
    h.cast_and_deliver(FLASH_HEAL, None);
    apply(&mut h.app, h.npc, h.caster, HAMMER_OF_JUSTICE);
    h.cast_and_deliver(FLASH_HEAL, None);
    let reasons: Vec<_> = h.inbox.failed.iter().map(|failed| failed.reason).collect();
    // Flash Heal has PreventionType 1 (silence); the stun is checked first.
    assert_eq!(
        reasons,
        vec![CastFailReason::Silenced, CastFailReason::Stunned]
    );
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
}

/// MOD_TOTAL_STAT_PERCENTAGE 137 +5%, MiscValueB 4 (Stamina).
const POWER_WORD_FORTITUDE: u32 = 21562;
/// MELEE_SLOW 193 +30% (haste).
const BLOODLUST: u32 = 2825;

#[test]
fn stat_auras_fold_into_character_stats_once() {
    let Some(mut h) = crate::spell_cast::test_harness::harness_with(
        Vec::new(),
        &[POWER_WORD_FORTITUDE, BLOODLUST, FLASH_HEAL],
        5,
        true,
    ) else {
        return;
    };
    h.advance(2);
    let stats = |h: &Harness| {
        *h.app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
    };
    let before = stats(&h);
    apply(&mut h.app, h.caster, h.caster, POWER_WORD_FORTITUDE);
    apply(&mut h.app, h.caster, h.caster, BLOODLUST);
    h.advance(2);
    let after = stats(&h);
    assert!(
        (after.primary.stamina - before.primary.stamina * 1.05).abs() < 1e-3,
        "{before:?} → {after:?}"
    );
    assert_eq!(after.primary.intellect, before.primary.intellect);
    assert!(
        (after.haste_pct - before.haste_pct - 30.0).abs() < 1e-3,
        "{before:?} → {after:?} {:?} {:?}",
        h.app.world().get::<crate::stats::StatModifiers>(h.caster),
        auras_of(&h, h.caster, BLOODLUST)
    );
    h.cast_and_deliver(FLASH_HEAL, None);
    let cast = h.app.world().get::<CastState>(h.caster).unwrap();
    // 1500 ms / (1 + haste%).
    let expected = (1500.0 / (1.0 + after.haste_pct / 100.0)).round() / 1000.0;
    assert!(
        (cast.duration - expected).abs() < 1e-3,
        "{} vs {expected}",
        cast.duration
    );
}
