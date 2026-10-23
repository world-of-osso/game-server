//! Source-qualified native item timing; no health writes, flags or seeded outcomes.
use super::venomfang::{equip_venomfang, unequip_venomfang};
use super::{
    DEVASTATE, GUILLOTINE_DAMAGE, GUILLOTINE_DRIVER, Harness, Inbox, PERFECTED_GUILLOTINE,
    item_fixture,
};
use shared::protocol::CombatLogKind;

const VENOMFANG: u32 = 1306635;
const VENOMFANG_BURST: u32 = 1306639;
const DAMAGE_SELF_HALF: u32 = 136890;
const FULL_HEAL_SELF: u32 = 17683;

fn set_native_health_case(h: &mut Harness, missing_half: bool) {
    // Native17683 effect67 heals missing HP;136890 effect165 self50%, CANT_CRIT
    // and IGNORE_DAMAGE_TAKEN_MODIFIERS. Source packet:guillotine-health-control-source.json.
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, FULL_HEAL_SELF, Some(h.npc), false)
        .unwrap();
    if missing_half {
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            h.npc,
            DAMAGE_SELF_HALF,
            Some(h.npc),
            false,
        )
        .unwrap();
    }
    let health = h
        .app
        .world()
        .get::<shared::components::Health>(h.npc)
        .unwrap();
    let expected = if missing_half { 0.5 } else { 1.0 };
    assert!((health.current / health.max - expected).abs() < 0.001);
}

fn wait_for_real_guillotine_flight(h: &mut Harness) -> f32 {
    for _ in 0..60 {
        h.cast_and_deliver(DEVASTATE, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let flying = h
            .app
            .world()
            .get_resource::<crate::spell_cast::missiles::PendingSpellHits>()
            .is_some_and(|pending| pending.has_caster(h.caster));
        if flying {
            let health = h
                .app
                .world()
                .get::<shared::components::Health>(h.npc)
                .unwrap();
            return 100.0 * (1.0 - health.current / health.max);
        }
        h.advance(40);
    }
    panic!("native5RPPM equipped driver never launched its actual one-second flight");
}

fn observe_noncritical_flight(h: &mut Harness, low_at_launch: bool) -> (f32, f32) {
    // Natural noncritical receipts isolate health timing from critical scaling;
    // never edit crit statistics, flags, chance or RNG. No critical-parity claim.
    for _ in 0..8 {
        set_native_health_case(h, low_at_launch);
        h.inbox = Inbox::default();
        let launch_missing = wait_for_real_guillotine_flight(h);
        set_native_health_case(h, !low_at_launch);
        h.advance(25);
        if let Some(hit) = h.inbox.log.iter().find(|event| {
            event.spell_id == Some(GUILLOTINE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
                && !event.crit
        }) {
            return (launch_missing, hit.amount as f32);
        }
        h.advance(40);
    }
    panic!("eight actual native flights produced no noncritical damage receipt");
}

#[test]
fn protection_producers_item_advanced_guillotine_uses_launch_not_impact_health() {
    let mut h = item_fixture(true);
    let percent = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == GUILLOTINE_DRIVER)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount
        / 100.0;
    let (full_missing, full_damage) = observe_noncritical_flight(&mut h, false);
    let (half_missing, half_damage) = observe_noncritical_flight(&mut h, true);
    assert!(full_missing < 1.0 && (49.0..51.0).contains(&half_missing));
    let ratio = (1.0 + percent * half_missing) / (1.0 + percent * full_missing);
    // Two integer rounding boundaries per hit, propagated through the ratio.
    let tolerance = 2.0 * (1.0 + ratio);
    assert!(
        (half_damage - full_damage * ratio).abs() <= tolerance,
        "launch-HP ratio={ratio}; full-launch/half-impact={full_damage}, half-launch/full-impact={half_damage}"
    );
}

fn assert_impact_poison_lifetime(h: &Harness, targets: &[bevy::prelude::Entity]) {
    for &target in targets {
        for (spell, periodic) in [(VENOMFANG, true), (VENOMFANG_BURST, false)] {
            assert!(
                h.inbox.log.iter().any(|event| event.spell_id == Some(spell)
                    && event.source == Some(h.caster.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.periodic == periodic
                    && event.amount > 0),
                "each retained native impact poison must tick and burst after weapon removal; spell{spell}, log={:?}",
                h.inbox.log
            );
        }
        assert!(!crate::auras::has_aura(h.app.world(), target, VENOMFANG));
    }
}

#[test]
#[ignore = "RED: perfected Guillotine lacks its guaranteed impact poison"]
fn protection_producers_item_advanced_perfected_impact_applies_owned_poison_to_both_targets() {
    let mut h = item_fixture(true);
    equip_venomfang(&mut h);
    assert!(crate::item_proc_data::has_equipped_bonus(h.app.world(), h.caster, 1291726).unwrap());
    let near = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 3.0);
    h.app
        .world_mut()
        .entity_mut(near)
        .insert(shared::components::Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        });
    // Native Attr4 BOUNCY_CHAIN_MISSILES delivers the two hits sequentially.
    let mut seen = Vec::new();
    let mut unequipped_in_flight = false;
    for frame in 0..4000 {
        if !unequipped_in_flight && frame % 43 == 0 {
            h.cast(DEVASTATE, Some(h.npc));
        }
        let previous = h.inbox.log.len();
        h.advance(1);
        let flying = h
            .app
            .world()
            .get_resource::<crate::spell_cast::missiles::PendingSpellHits>()
            .is_some_and(|pending| pending.has_caster(h.caster));
        if !unequipped_in_flight && flying {
            assert!(
                !h.inbox
                    .log
                    .iter()
                    .any(|event| event.spell_id == Some(PERFECTED_GUILLOTINE)
                        && event.kind == CombatLogKind::Damage),
                "unequip must precede the first native impact"
            );
            // Remove the real driver before impact: independent5RPPM cannot mask
            // a missing guaranteed child. No proc chance, rows or RNG are edited.
            unequip_venomfang(&mut h);
            unequipped_in_flight = true;
        }
        let receipts = &h.inbox.log[previous..];
        let hits: Vec<_> = receipts
            .iter()
            .filter(|event| {
                event.spell_id == Some(PERFECTED_GUILLOTINE)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .collect();
        if hits.is_empty() {
            continue;
        }
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        for hit in hits {
            let target = hit.target.expect("native damage has its recipient");
            assert!([h.npc.to_bits(), near.to_bits()].contains(&target));
            assert!(
                receipts
                    .iter()
                    .any(|event| event.spell_id == Some(VENOMFANG)
                        && event.source == Some(h.caster.to_bits())
                        && event.target == Some(target)
                        && matches!(
                            event.kind,
                            CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                        )),
                "each actual perfected impact must apply owned poison in its frame, not rely on an old independent proc; receipts={receipts:?}"
            );
            if !seen.contains(&target) {
                seen.push(target);
            }
        }
        if seen.len() == 2 {
            assert!(unequipped_in_flight);
            h.inbox = Inbox::default();
            h.advance(140);
            assert_impact_poison_lifetime(&h, &[h.npc, near]);
            return;
        }
    }
    panic!("native two-piece never produced a perfected Guillotine impact");
}
