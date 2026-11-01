//! Equipped Resonant Bellowstone: SimC76f8118 unique_gear_midnight.cpp2109.
use super::{Harness, devastate, item_fixture};
use crate::auras::{self, ActiveAuras, procs};
use shared::protocol::EquipmentSlot;

const ITEM: u32 = 250228;
const DRIVER: u32 = 1250564;
const LAST_ROAR: u32 = 1254180;
const ECHOING_ROAR: u32 = 1254331;

fn emit(h: &mut Harness, hit_mask: u32) {
    procs::push_event(
        h.app.world_mut(),
        procs::AuraEvent {
            actor: h.caster,
            target: Some(h.npc),
            action_target: None,
            actor_mask: [procs::PROC_DEAL_MELEE_ABILITY, 0],
            target_mask: [0; 2],
            spell_id: None,
            can_proc: true,
            cast_used_proc_modifier: false,
            periodic: false,
            damage: 100.0,
            heal: None,
            hit_mask,
            reflected: false,
        },
    );
    h.advance(1);
}

fn crit(h: &Harness) -> f32 {
    h.app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct
}

fn aura(h: &Harness, spell: u32) -> Option<auras::AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)?
        .iter()
        .find(|a| a.spell_id == spell)
        .cloned()
}

#[test]
fn equipped_bellowstone_applies_item_scaled_crit_and_stacks_only_on_crits() {
    let _seed = crate::random::seed_scope(70448);
    let mut h = item_fixture(false);
    devastate(&mut h);
    h.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItems>(h.caster)
        .unwrap()
        .items
        .push((
            EquipmentSlot::Trinket1,
            shared::item_data::ItemDefinitionSource::Retail,
            ITEM,
        ));
    h.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItemModifiers>(h.caster)
        .unwrap()
        .items
        .push((EquipmentSlot::Trinket1, Default::default()));
    h.advance(3);
    let driver = aura(&h, DRIVER).expect("equipped trinket supplies its native driver");
    assert!(driver.cast_item_level.is_some());
    let base = crit(&h);
    for _ in 0..100 {
        devastate(&mut h);
        if aura(&h, LAST_ROAR).is_some() {
            break;
        }
    }
    let last = aura(&h, LAST_ROAR).expect("real equipment proc applies Last Roar");
    assert!(last.effects.iter().any(|e| e.index == 0 && e.amount > 0.0));
    assert!(crit(&h) > base, "Last Roar must increase effective crit");
    let stacks = aura(&h, ECHOING_ROAR).map_or(0, |echo| echo.stacks);
    emit(&mut h, procs::PROC_HIT_NORMAL);
    assert_eq!(
        aura(&h, ECHOING_ROAR).map_or(0, |echo| echo.stacks),
        stacks,
        "normal hits cannot trigger Echoing Roar"
    );
    let before = crit(&h);
    emit(&mut h, procs::PROC_HIT_CRITICAL);
    let echo = aura(&h, ECHOING_ROAR).expect("critical hit adds Echoing Roar");
    assert_eq!(echo.stacks, stacks + 1);
    assert!(echo.effects.iter().any(|e| e.index == 0 && e.amount > 0.0));
    assert!(
        crit(&h) > before,
        "Echoing Roar must increase effective crit"
    );
    let first = crit(&h);
    emit(&mut h, procs::PROC_HIT_NORMAL);
    assert_eq!(aura(&h, ECHOING_ROAR).unwrap().stacks, stacks + 1);
    emit(&mut h, procs::PROC_HIT_CRITICAL);
    assert_eq!(aura(&h, ECHOING_ROAR).unwrap().stacks, stacks + 2);
    assert!(crit(&h) > first, "each critical stack increases crit");
}
