//! Mainline 12.1.0.69933 Barbed Shot totals and Stormkeeper uses.
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
fn dummy20_barbed_reapplication_adds_unpaid_damage_to_native_five_tick_total() {
    let mut h = fixture(&[217200], 3);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, npc, 217200);
    h.advance(45);
    let paid = damage(&h, 217200);
    assert_eq!(paid.len(), 2, "extra initial and first regular native tick");
    let base = paid[0];
    assert_eq!(paid[1], base);
    h.inbox.log.clear();
    apply(&mut h, caster, npc, 217200);
    h.advance(170);
    let rolled = damage(&h, 217200);
    assert_eq!(
        rolled.len(),
        4,
        "native refresh preserves regular tick phase"
    );
    assert_eq!(
        rolled.iter().sum::<i32>(),
        base * 8,
        "new five-tick budget plus three unpaid old ticks"
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
