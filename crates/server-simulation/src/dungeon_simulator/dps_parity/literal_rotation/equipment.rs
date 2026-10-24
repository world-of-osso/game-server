//! Original equipped native identities, item-use context and shared APL operands.
#[path = "equipment_metadata.rs"]
mod metadata;
#[path = "equipment_sets.rs"]
mod sets;
use crate::item_templates::{ItemTemplates, OnUseSpell};
use crate::simc_apl_native::{Bindings, NativeFacts};
use crate::stats::{EquippedItemModifiers, EquippedItems, NativeItemInstances};
use bevy::prelude::*;
use metadata::Metadata;
use server::simc_apl::Action;
use shared::item_data::ItemDefinitionSource;
use shared::protocol::EquipmentSlot;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[cfg(test)]
#[path = "equipment_tests.rs"]
mod tests;

// Pinned SimC use_items preference, independent of map ordering.
const SLOTS: &[(&str, EquipmentSlot)] = &[
    ("trinket1", EquipmentSlot::Trinket1),
    ("trinket2", EquipmentSlot::Trinket2),
    ("main_hand", EquipmentSlot::MainHand),
];

#[derive(Clone, Copy)]
pub(crate) struct ItemUse {
    pub item_id: u32,
    pub on_use: OnUseSpell,
    pub item_level: u16,
    stat_buff: bool,
}

struct Slot {
    item_id: u32,
    native_slot: EquipmentSlot,
    on_use: Option<ItemUse>,
    metadata: Metadata,
}

impl Slot {
    fn empty(native_slot: EquipmentSlot) -> Self {
        Self {
            item_id: 0,
            native_slot,
            on_use: None,
            metadata: Metadata::default(),
        }
    }
}

pub(crate) struct Equipment {
    slots: BTreeMap<String, Slot>,
    equipped: BTreeSet<u32>,
    names: Arc<BTreeMap<String, BTreeSet<u32>>>,
    set_facts: BTreeMap<String, bool>,
}

impl Equipment {
    pub fn load(world: &World, actor: Entity, bindings: &mut Bindings) -> Result<Self, String> {
        let equipped = read_equipped_ids(world, actor)?;
        let mut slots = BTreeMap::new();
        for &(name, slot) in SLOTS {
            let entry = read_slot(world, actor, slot)?;
            bindings.actions.remove(name);
            if let Some(item) = entry.on_use {
                bindings.actions.insert(name.into(), item.on_use.spell_id);
            }
            slots.insert(name.into(), entry);
        }
        Ok(Self {
            slots,
            equipped,
            names: Arc::clone(&world.resource::<ItemTemplates>().item_names),
            set_facts: sets::read_set_facts(world, actor)?,
        })
    }

    pub fn uses(&self) -> impl Iterator<Item = (&str, ItemUse)> + '_ {
        SLOTS
            .iter()
            .filter_map(move |&(name, _)| self.slots[name].on_use.map(|item| (name, item)))
    }

    pub fn slot_for_action(&self, action: &Action) -> Result<Option<&str>, String> {
        if let Some(slot) = action.options.get("slot") {
            return self
                .slots
                .get_key_value(slot)
                .map(|(key, _)| Some(key.as_str()))
                .ok_or_else(|| format!("unknown literal item slot {slot}"));
        }
        let name = action
            .options
            .get("name")
            .ok_or("literal item action lacks slot/name")?;
        let ids = self.named_ids(name)?;
        Ok(self
            .uses()
            .find(|(_, item)| ids.contains(&item.item_id))
            .map(|(slot, _)| slot))
    }

    pub fn fact(&self, name: &str, native: &NativeFacts<'_>) -> Result<f64, String> {
        if let Some(item) = name.strip_prefix("equipped.") {
            return self.equipped_fact(item);
        }
        if name.starts_with("set_bonus.") {
            return self
                .set_facts
                .get(name)
                .copied()
                .map(boolean)
                .ok_or_else(|| format!("unknown native set-bonus operand {name}"));
        }
        let tail = name
            .strip_prefix("trinket.")
            .ok_or("literal trinket prefix absent")?;
        let (index, field) = tail
            .split_once('.')
            .ok_or("literal trinket requires a field")?;
        let slot = match index {
            "1" => "trinket1",
            "2" => "trinket2",
            _ => return Err(format!("unknown literal trinket {index}")),
        };
        self.trinket_fact(&self.slots[slot], field, native)
    }

    fn trinket_fact(
        &self,
        item: &Slot,
        field: &str,
        native: &NativeFacts<'_>,
    ) -> Result<f64, String> {
        match field {
            "has_use_buff" => Ok(boolean(item.on_use.is_some_and(|item| item.stat_buff))),
            "has_cooldown" => Ok(boolean(!item.metadata.cooldowns.is_empty())),
            "cooldown.duration" => Ok(item
                .metadata
                .cooldowns
                .iter()
                .map(|row| row.duration)
                .fold(0.0, f64::max)),
            "cooldown.remains" | "cooldown.ready" => read_cooldown(item, field, native),
            "proc.any_dps.duration" => Ok(item.metadata.duration),
            _ if field.starts_with("has_buff.") => stat_fact(&item.metadata, &field[9..]),
            _ if field.starts_with("is.") => Ok(boolean(
                self.named_ids(&field[3..])?.contains(&item.item_id),
            )),
            _ => Err(format!("unknown literal trinket fact {field}")),
        }
    }

    fn equipped_fact(&self, item: &str) -> Result<f64, String> {
        let equipped = if let Ok(id) = item.parse::<u32>() {
            self.equipped.contains(&id)
        } else {
            self.named_ids(item)?
                .iter()
                .any(|id| self.equipped.contains(id))
        };
        Ok(boolean(equipped))
    }

    fn named_ids(&self, name: &str) -> Result<&BTreeSet<u32>, String> {
        self.names
            .get(name)
            .ok_or_else(|| format!("unknown native item name {name}"))
    }
}

fn boolean(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn stat_fact(metadata: &Metadata, name: &str) -> Result<f64, String> {
    if !matches!(
        name,
        "strength" | "agility" | "intellect" | "mastery" | "versatility" | "haste" | "crit"
    ) {
        return Err(format!("unknown native item buff stat {name}"));
    }
    Ok(boolean(metadata.stats.contains(name)))
}

fn read_cooldown(item: &Slot, field: &str, native: &NativeFacts<'_>) -> Result<f64, String> {
    if item.metadata.cooldowns.is_empty() {
        return Ok(0.0);
    }
    let mut remains: f64 = 0.0;
    for cooldown in &item.metadata.cooldowns {
        let remaining = if cooldown.proc {
            native.equipment_proc_cooldown(cooldown.spell.id, item.native_slot)
        } else {
            native.cooldown_for_spell(&cooldown.spell, "remains")?
        };
        remains = remains.max(remaining);
    }
    Ok(if field == "cooldown.ready" {
        boolean(remains == 0.0)
    } else {
        remains
    })
}

fn read_equipped_ids(world: &World, actor: Entity) -> Result<BTreeSet<u32>, String> {
    let equipped = world
        .get::<EquippedItems>(actor)
        .ok_or("native equipped items absent")?;
    equipped
        .items
        .iter()
        .map(|&(_, source, id)| {
            if source != ItemDefinitionSource::Retail {
                return Err(format!(
                    "literal native equipment namespace {source:?} is unqualified"
                ));
            }
            Ok(id)
        })
        .collect()
}

fn read_slot(world: &World, actor: Entity, slot: EquipmentSlot) -> Result<Slot, String> {
    let equipped = world
        .get::<EquippedItems>(actor)
        .ok_or("native equipped items absent")?;
    let Some(&(_, source, item_id)) = equipped.items.iter().find(|(found, _, _)| *found == slot)
    else {
        return Ok(Slot::empty(slot));
    };
    read_slot_item(world, actor, slot, source, item_id)
}

fn read_slot_item(
    world: &World,
    actor: Entity,
    slot: EquipmentSlot,
    source: ItemDefinitionSource,
    item_id: u32,
) -> Result<Slot, String> {
    let template = world
        .resource::<ItemTemplates>()
        .get(source, item_id)
        .ok_or_else(|| format!("native item{item_id} definition absent"))?;
    let properties = read_modifiers(world, actor, slot)?;
    let native = world.resource::<NativeItemInstances>();
    let modified = server::item_instance::bonuses::apply_item_bonuses(
        &template.scaling,
        &properties.bonus_list,
        &native.bonuses,
    )?;
    let metadata = read_slot_metadata(world, item_id, &modified.effect_ids)?;
    let on_use = read_item_use(
        world,
        item_id,
        template,
        properties,
        &modified,
        metadata.use_buff,
    )?;
    Ok(Slot {
        item_id,
        native_slot: slot,
        on_use,
        metadata,
    })
}

fn read_slot_metadata(
    world: &World,
    item_id: u32,
    bonus_effects: &[u32],
) -> Result<Metadata, String> {
    let native = world.resource::<NativeItemInstances>();
    let effects = server::item_instance::effects::equipped_item_effects(
        item_id,
        bonus_effects,
        &native.effects,
    )?;
    Metadata::load(world.resource::<crate::spell_info::SpellStore>(), &effects)
}

fn read_item_use(
    world: &World,
    item_id: u32,
    template: &crate::item_templates::ItemTemplate,
    properties: &server::item_instance::ItemInstanceModifiers,
    modified: &server::item_instance::bonuses::BonusResult,
    stat_buff: bool,
) -> Result<Option<ItemUse>, String> {
    let Some(on_use) = template.on_use else {
        return Ok(None);
    };
    let item_level = server::item_instance::item_instance_stats(
        &modified.scaling,
        template.inventory_type,
        properties,
        world.resource::<crate::item_stats::ItemScalingTables>(),
    )?
    .item_level;
    Ok(Some(ItemUse {
        item_id,
        on_use,
        item_level,
        stat_buff,
    }))
}

fn read_modifiers(
    world: &World,
    actor: Entity,
    slot: EquipmentSlot,
) -> Result<&server::item_instance::ItemInstanceModifiers, String> {
    world
        .get::<EquippedItemModifiers>(actor)
        .ok_or("native instance modifiers absent")?
        .items
        .iter()
        .find(|(found, _)| *found == slot)
        .map(|(_, properties)| properties)
        .ok_or_else(|| format!("native instance modifiers for {slot:?} absent"))
}
