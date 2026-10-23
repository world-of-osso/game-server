//! SimC76f8118 unique_gear_midnight.cpp6279–6383, native70448 equipped sources.
use super::{DEVASTATE, Harness, Inbox, devastate, item_fixture};
use shared::protocol::{CombatLogKind, EquipmentSlot};

const VENOMFANG_ITEM: u32 = 268209;
const VENOMFANG_DRIVER: u32 = 1291718;
const VENOMFANG: u32 = 1306635;
const VENOMFANG_BURST: u32 = 1306639;

pub(super) fn equip_venomfang(h: &mut Harness) {
    h.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItems>(h.caster)
        .unwrap()
        .items
        .push((
            EquipmentSlot::MainHand,
            shared::item_data::ItemDefinitionSource::Retail,
            VENOMFANG_ITEM,
        ));
    h.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItemModifiers>(h.caster)
        .unwrap()
        .items
        .push((EquipmentSlot::MainHand, Default::default()));
    h.advance(3);
    let driver = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == VENOMFANG_DRIVER)
        .expect("actual equipped weapon must install its driver");
    assert_eq!(driver.cast_item_level, Some(219));
    for index in [0, 1] {
        assert!(
            driver
                .effects
                .iter()
                .any(|effect| effect.index == index && effect.amount > 0.0)
        );
    }
}

fn poison_damage(h: &Harness, spell: u32) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(spell) && event.kind == CombatLogKind::Damage && event.amount > 0
        })
        .count()
}

pub(super) fn unequip_venomfang(h: &mut Harness) {
    h.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItems>(h.caster)
        .unwrap()
        .items
        .retain(|(slot, _, _)| *slot != EquipmentSlot::MainHand);
    h.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItemModifiers>(h.caster)
        .unwrap()
        .items
        .retain(|(slot, _)| *slot != EquipmentSlot::MainHand);
    h.advance(3);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        VENOMFANG_DRIVER
    ));
}

#[test]
fn protection_producers_venomfang_equipped_poison_ticks_then_bursts_after_unequip() {
    let mut h = item_fixture(false);
    equip_venomfang(&mut h);
    for _ in 0..60 {
        devastate(&mut h);
        if poison_damage(&h, VENOMFANG) > 0 {
            break;
        }
    }
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(DEVASTATE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(
        poison_damage(&h, VENOMFANG) > 0,
        "real equipped driver must produce owned periodic damage"
    );
    assert!(h.inbox.log.iter().filter(|event| event.spell_id == Some(VENOMFANG)
        && event.kind == CombatLogKind::Damage).all(|event| event.periodic
        && event.source == Some(h.caster.to_bits()) && event.target == Some(h.npc.to_bits())));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(VENOMFANG_BURST)),
        "poison burst belongs to stack expiry, not application; log={:?}",
        h.inbox.log
    );
    unequip_venomfang(&mut h);
    h.inbox = Inbox::default();
    h.advance(140);
    assert!(
        poison_damage(&h, VENOMFANG_BURST) > 0,
        "last poison stack must burst with its original item-scaled amount after unequip"
    );
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, VENOMFANG));
    h.inbox = Inbox::default();
    for _ in 0..20 {
        devastate(&mut h);
    }
    assert_eq!(poison_damage(&h, VENOMFANG), 0);
    assert_eq!(poison_damage(&h, VENOMFANG_BURST), 0);
}

#[test]
fn protection_producers_venomfang_partial_then_last_native_stack_expiries_each_burst() {
    let mut h = item_fixture(false);
    equip_venomfang(&mut h);
    let mut overlapping = false;
    for frame in 0..4000 {
        if frame % 43 == 0 {
            h.cast(DEVASTATE, Some(h.npc));
        }
        h.advance(1);
        let stacks = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == VENOMFANG))
            .map_or(0, |aura| aura.stacks);
        if stacks == 2 {
            overlapping = true;
            break;
        }
    }
    assert!(
        overlapping,
        "real native5RPPM attacks must supply two overlapping applications"
    );
    let before = poison_damage(&h, VENOMFANG_BURST);
    unequip_venomfang(&mut h);
    let mut partial = false;
    for _ in 0..120 {
        let stacks = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == VENOMFANG))
            .map_or(0, |aura| aura.stacks);
        if stacks == 1 {
            partial = true;
            break;
        }
        h.advance(1);
    }
    assert!(
        partial,
        "first independent expiry must leave the later poison application alive"
    );
    assert_eq!(
        poison_damage(&h, VENOMFANG_BURST),
        before + 1,
        "partial expiry emits one real burst before whole-aura removal"
    );
    h.advance(140);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, VENOMFANG));
    assert_eq!(
        poison_damage(&h, VENOMFANG_BURST),
        before + 2,
        "last expiry emits exactly one additional burst, not a duplicate remove callback"
    );
}

#[test]
fn protection_producers_venomfang_catalog_without_weapon_cannot_proc() {
    let mut h = item_fixture(false);
    for _ in 0..60 {
        devastate(&mut h);
    }
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(DEVASTATE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| matches!(event.spell_id, Some(VENOMFANG | VENOMFANG_BURST)))
    );
}
