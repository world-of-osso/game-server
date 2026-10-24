//! Native DB2 Barbed per-tick amounts and Stormkeeper uses.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn fixture(ids: &[u32], class: u8) -> Harness {
    let spells = ids
        .iter()
        .map(|id| {
            let mut spell = world_db().unwrap().store.get(*id).unwrap().clone();
            for effect in &mut spell.effects {
                effect.variance = 0.0;
            }
            spell
        })
        .collect();
    harness(spells, ids, class).unwrap()
}
fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
fn execute(h: &mut Harness, id: u32, target: Entity) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, Some(target)).unwrap();
}
fn damage(h: &Harness, id: u32) -> Vec<i32> {
    h.inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(id) && e.kind == CombatLogKind::Damage && e.periodic)
        .map(|e| e.amount)
        .collect()
}
#[test]
fn dummy20_barbed_reapplication_adds_unpaid_damage_to_native_per_tick_amount() {
    let mut h = fixture(&[217200], 3);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, npc, 217200);
    h.advance(45);
    let paid = damage(&h, 217200);
    assert_eq!(paid.len(), 2, "extra initial and first regular native tick");
    assert_eq!(
        paid,
        vec![86, 86],
        "native0.857 AP per payment, not the tooltip's dummy5 divided over the clock"
    );
    h.inbox.log.clear();
    apply(&mut h, caster, npc, 217200);
    h.advance(250);
    let rolled = damage(&h, 217200);
    assert_eq!(
        rolled.len(),
        7,
        "native twelve-second nonstacking recast resets its extra-initial schedule"
    );
    assert_eq!(
        rolled.iter().sum::<i32>(),
        1029,
        "fresh85.7 per tick plus five unpaid85.7 ticks over seven fresh payments"
    );
}
#[test]
fn dummy20_barbed_completed_cast_grants_native_periodic_focus_child() {
    let mut h = fixture(&[217200, 246152], 3);
    h.set_power(PowerType::Focus, 0.0);
    let npc = h.npc;
    execute(&mut h, 217200, npc);
    h.advance(180);
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(246152) && e.kind == CombatLogKind::Energize)
        .collect();
    assert_eq!(grants.len(), 4);
    assert!(
        grants
            .iter()
            .all(|e| e.target == Some(h.caster.to_bits()) && e.periodic)
    );
    assert_eq!(grants.iter().map(|e| e.amount).sum::<i32>(), 20);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 246152));
}
fn stormkeeper_uses(id: u32) {
    let mut h = fixture(&[191634, 188196, 188443, 188389], 7);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, caster, 191634);
    execute(&mut h, 188389, npc);
    h.advance(4);
    assert!(
        crate::auras::has_aura(h.app.world(), caster, 191634),
        "unrelated Flame Shock must not consume a use"
    );
    h.advance(35);
    h.cast_and_deliver(id, Some(npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox.go.iter().any(|go| go.spell_id == id),
        "first empowered bolt is instant"
    );
    h.advance(35);
    assert!(
        crate::auras::has_aura(h.app.world(), caster, 191634),
        "one of two authored uses remains after real hit/proc processing"
    );
    h.cast_and_deliver(id, Some(npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.go.iter().filter(|go| go.spell_id == id).count(), 2);
    h.advance(35);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 191634));
}
#[test]
fn dummy20_stormkeeper_two_instant_lightning_bolts() {
    stormkeeper_uses(188196);
}
#[test]
fn dummy20_stormkeeper_two_instant_chain_lightnings() {
    stormkeeper_uses(188443);
}

#[test]
fn dummy20_barbed_native_clock_pays_db2_amount_each_tick_and_expires() {
    let mut h = fixture(&[217200], 3);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, npc, 217200);
    h.advance(250);
    assert_eq!(damage(&h, 217200), vec![86; 7]);
    assert!(!crate::auras::has_aura(h.app.world(), npc, 217200));
    h.advance(50);
    assert_eq!(damage(&h, 217200).len(), 7);
}

#[test]
fn dummy20_barbed_foreign_owner_gets_only_its_fresh_budget() {
    let mut h = fixture(&[217200], 3);
    let caster = h.caster;
    let npc = h.npc;
    let other = spawn_player(&mut h.app, 3, 2.0);
    h.advance(1);
    apply(&mut h, caster, npc, 217200);
    h.advance(45);
    apply(&mut h, other, npc, 217200);
    h.advance(250);
    let owned: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(217200)
                && e.kind == CombatLogKind::Damage
                && e.source == Some(caster.to_bits())
        })
        .map(|e| e.amount)
        .collect();
    assert_eq!(owned, vec![86; 7]);
    // The second player has no authenticated connection; health proves both
    // independent budgets without requiring its private log in our inbox.
    assert_eq!(h.health(npc), 5000.0 - 14.0 * 86.0);
}

#[test]
fn dummy20_barbed_refresh_resnapshots_new_power_without_rescaling_unpaid_old_damage() {
    let mut h = fixture(&[217200], 3);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, npc, 217200);
    h.advance(45);
    assert_eq!(damage(&h, 217200), vec![86; 2]);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .ranged_attack_power = 200.0;
    h.inbox.log.clear();
    apply(&mut h, caster, npc, 217200);
    h.advance(250);
    assert_eq!(damage(&h, 217200), vec![233; 7]);
}

#[test]
fn dummy20_barbed_rejected_friendly_target_does_not_start_focus() {
    let mut h = fixture(&[217200, 246152], 3);
    let caster = h.caster;
    h.cast_and_deliver(217200, Some(caster));
    h.advance(180);
    assert_eq!(h.inbox.failed.len(), 1);
    assert!(!h.inbox.log.iter().any(|e| e.spell_id == Some(246152)));
}

#[test]
fn dummy20_stormkeeper_root_completion_and_cancellation() {
    for cancel in [false, true] {
        let mut h = fixture(&[191634, 188196], 7);
        let caster = h.caster;
        let npc = h.npc;
        h.cast_and_deliver(191634, Some(caster));
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(caster)
                .is_some()
        );
        assert!(!crate::auras::has_aura(h.app.world(), caster, 191634));
        if cancel {
            super::cancel_cast(h.app.world_mut(), caster);
        }
        h.advance(50);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            crate::auras::has_aura(h.app.world(), caster, 191634),
            !cancel
        );
        if cancel {
            continue;
        }
        for remaining in [true, false] {
            h.cast_and_deliver(188196, Some(npc));
            assert!(
                h.app
                    .world()
                    .get::<shared::casting::CastState>(caster)
                    .is_none()
            );
            h.advance(35);
            assert_eq!(
                crate::auras::has_aura(h.app.world(), caster, 191634),
                remaining
            );
        }
        h.cast_and_deliver(188196, Some(npc));
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(caster)
                .is_some(),
            "third bolt resumes its native cast time"
        );
        h.advance(80);
        assert_eq!(
            h.inbox.go.iter().filter(|go| go.spell_id == 188196).count(),
            3
        );
    }
}
