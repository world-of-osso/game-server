use super::equipment::*;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Program};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[test]
fn healer_item_rate_fact_qualifies_soulcoiler_but_not_idol_or_unknown_items() {
    let mut world = World::new();
    let actor = world.spawn_empty().id();
    let bindings = Bindings::default();
    let native = NativeFacts::new(&world, actor, &bindings, Encounter::dungeon(0.0), 1);
    let mut equipment = Equipment {
        slots: BTreeMap::new(),
        equipped: BTreeSet::new(),
        names: Arc::new(BTreeMap::new()),
        set_facts: BTreeMap::new(),
    };
    let program = Program::parse(&[("default", "use_items")]).unwrap();
    let action = Action {
        name: "use_items".into(),
        options: [("slot".into(), "trinket1".into())].into(),
        source_row: None,
        target: None,
    };
    for (item_id, expected) in [(270162, 1.0), (270169, 0.0), (0, 0.0)] {
        let mut slot = Slot::empty(shared::protocol::EquipmentSlot::Trinket1);
        slot.item_id = item_id;
        equipment.slots.insert("trinket1".into(), slot);
        assert_eq!(
            equipment.fact("trinket.1.healer_rate_qualified", &native),
            Ok(expected),
            "item {item_id}"
        );
        let context = super::item_context::ItemContext {
            facts: &native,
            equipment: &equipment,
            program: &program,
        };
        assert_eq!(
            context.value_for(&action, "this_trinket.healer_rate_qualified"),
            Ok(expected),
            "item {item_id} must work without a DPS-specific equipment facts wrapper"
        );
    }
}
