//! TC a352b1fa SpellAuras.cpp486–494/863–899: native per-second SpellPower costs.
//! Each aura charges its caster one second after application, then every second;
//! a caster that cannot pay loses the aura and its channel.
use super::test_harness::*;
use shared::casting::CastState;
use shared::components::{Mana, PowerType};
use shared::protocol::CombatLogKind;

const DRAIN_LIFE: u32 = 234153;
const CRACKLING_JADE_LIGHTNING: u32 = 117952;
const WINDWALKER_MONK: u32 = 137025;
const CHAOS_SALVO: u32 = 432569;

fn channeling(h: &Harness, spell: u32) -> bool {
    h.app
        .world()
        .get::<CastState>(h.caster)
        .is_some_and(|cast| cast.spell_id == spell)
}

fn mana(h: &Harness) -> f32 {
    h.app.world().get::<Mana>(h.caster).unwrap().current
}

fn periodic_events(h: &Harness, spell: u32, kind: CombatLogKind) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|event| event.spell_id == Some(spell) && event.kind == kind && event.periodic)
        .count()
}

/// Frames from the cast frame to 0.9s, and across the first charge to 1.1s.
const BEFORE_FIRST_CHARGE: usize = 15;
const ACROSS_FIRST_CHARGE: usize = 4;

#[test]
fn drain_life_pays_three_percent_max_mana_each_second_or_ends() {
    for short in [false, true] {
        let mut h = harness(Vec::new(), &[DRAIN_LIFE], 9).expect("real world.db required");
        let npc = h.npc;
        let max = h.app.world().get::<Mana>(h.caster).unwrap().max;
        let cost = (max * 0.03) as i32 as f32;
        h.cast_and_deliver(DRAIN_LIFE, Some(npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        h.advance(BEFORE_FIRST_CHARGE);
        if short {
            h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = cost / 2.0;
        }
        let before = mana(&h);
        h.advance(ACROSS_FIRST_CHARGE);
        if short {
            // Removed before its periodic: no heal tick and nothing paid.
            assert!(!channeling(&h, DRAIN_LIFE));
            assert!(!crate::auras::has_aura(h.app.world(), npc, DRAIN_LIFE));
            assert_eq!(periodic_events(&h, DRAIN_LIFE, CombatLogKind::Heal), 0);
            assert!(mana(&h) >= before);
        } else {
            assert!(channeling(&h, DRAIN_LIFE));
            assert_eq!(periodic_events(&h, DRAIN_LIFE, CombatLogKind::Heal), 1);
            // One frame of base regeneration may land after the charge.
            let paid = before - mana(&h);
            assert!(paid > cost * 0.9 && paid <= cost, "paid {paid} of {cost}");
        }
    }
}

#[test]
fn crackling_jade_lightning_drains_twenty_energy_each_second_or_ends() {
    for short in [false, true] {
        let mut h = harness(Vec::new(), &[CRACKLING_JADE_LIGHTNING, WINDWALKER_MONK], 10)
            .expect("real world.db required");
        let caster = h.caster;
        super::cast_triggered(h.app.world_mut(), caster, WINDWALKER_MONK, Some(caster), 1);
        h.advance(1);
        assert!(crate::auras::has_aura(
            h.app.world(),
            caster,
            WINDWALKER_MONK
        ));
        h.set_power(PowerType::Energy, 100.0);
        let npc = h.npc;
        h.cast_and_deliver(CRACKLING_JADE_LIGHTNING, Some(npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        h.advance(BEFORE_FIRST_CHARGE);
        if short {
            h.set_power(PowerType::Energy, 10.0);
        }
        let before = h.power(PowerType::Energy);
        h.advance(ACROSS_FIRST_CHARGE);
        let after = h.power(PowerType::Energy);
        if short {
            assert!(!channeling(&h, CRACKLING_JADE_LIGHTNING));
            assert!(!crate::auras::has_aura(
                h.app.world(),
                npc,
                CRACKLING_JADE_LIGHTNING
            ));
            assert!(after >= before, "nothing paid: {before} -> {after}");
        } else {
            // NO_TARGET_PER_SECOND_COSTS: only the caster's own aura charges, once.
            assert!(channeling(&h, CRACKLING_JADE_LIGHTNING));
            let paid = before - after;
            assert!(paid > 15.0 && paid <= 20.0, "paid {paid}");
        }
    }
}

#[test]
fn chaos_salvo_charges_each_unaffected_aura_ten_energy_each_second() {
    for short in [false, true] {
        let mut h = harness(Vec::new(), &[CHAOS_SALVO], 4).expect("real world.db required");
        let (caster, npc) = (h.caster, h.npc);
        h.set_power(PowerType::Energy, 100.0);
        assert!(super::start_triggered_channel(
            h.app.world_mut(),
            caster,
            CHAOS_SALVO,
            npc
        ));
        h.advance(BEFORE_FIRST_CHARGE + 3);
        if short {
            h.set_power(PowerType::Energy, 15.0);
        }
        let before = h.power(PowerType::Energy);
        h.advance(ACROSS_FIRST_CHARGE);
        let after = h.power(PowerType::Energy);
        if short {
            // The first aura's 10 leaves too little for the second.
            assert!(!channeling(&h, CHAOS_SALVO));
            assert!(!crate::auras::has_aura(h.app.world(), npc, CHAOS_SALVO));
            assert!(!crate::auras::has_aura(h.app.world(), caster, CHAOS_SALVO));
        } else {
            // Target and caster auras each pay: ATTR2 0x800 is clear.
            assert!(channeling(&h, CHAOS_SALVO));
            let paid = before - after;
            assert!(paid > 15.0 && paid <= 20.0, "paid {paid}");
        }
    }
}
