//! Equipped native item identities, on-use context and literal trinket operands.
use crate::item_templates::{ItemTemplates, OnUseSpell};
use crate::simc_apl_native::{Bindings, NativeFacts};
use crate::stats::{EquippedItemModifiers, EquippedItems, NativeItemInstances};
use bevy::prelude::*;
use server::simc_apl::{Action, Facts};
use shared::protocol::EquipmentSlot;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "equipment_tests.rs"]
mod tests;

const ALGETHAR_PUZZLE_BOX: u32 = 193701;

#[derive(Clone, Copy)]
pub(crate) struct ItemUse {
    pub item_id: u32,
    pub on_use: OnUseSpell,
    pub item_level: u16,
    stat_buff: bool,
}

struct Slot {
    item_id: u32,
    on_use: Option<ItemUse>,
}

pub(crate) struct Equipment {
    slots: BTreeMap<String, Slot>,
}

impl Equipment {
    pub fn load(world: &World, actor: Entity, bindings: &mut Bindings) -> Result<Self, String> {
        let mut slots = BTreeMap::new();
        for (name, slot) in [
            ("main_hand", EquipmentSlot::MainHand),
            ("trinket1", EquipmentSlot::Trinket1),
            ("trinket2", EquipmentSlot::Trinket2),
        ] {
            let entry = read_slot(world, actor, slot)?;
            if let Some(item) = entry.on_use {
                bindings.actions.insert(name.into(), item.on_use.spell_id);
            }
            slots.insert(name.into(), entry);
        }
        Ok(Self { slots })
    }

    pub fn uses(&self) -> impl Iterator<Item = (&str, ItemUse)> + '_ {
        self.slots
            .iter()
            .filter_map(|(name, slot)| slot.on_use.map(|item| (name.as_str(), item)))
    }

    pub fn slot_for_action(&self, action: &Action) -> Result<Option<&str>, String> {
        if let Some(slot) = action.options.get("slot") {
            return self
                .slots
                .get_key_value(slot)
                .map(|(key, _)| Some(key.as_str()))
                .ok_or_else(|| format!("unknown literal item slot {slot}"));
        }
        match action.options.get("name").map(String::as_str) {
            Some("algethar_puzzle_box") => Ok(self
                .slots
                .iter()
                .find(|(_, slot)| slot.item_id == ALGETHAR_PUZZLE_BOX)
                .map(|(name, _)| name.as_str())),
            name => Err(format!("unknown literal named item {name:?}")),
        }
    }

    pub fn fact(&self, name: &str, native: &NativeFacts<'_>) -> Result<f64, String> {
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
        let item = self
            .slots
            .get(slot)
            .ok_or_else(|| format!("native equipped {slot} absent"))?;
        match field {
            "has_use_buff" => Ok(if item.on_use.is_some_and(|item| item.stat_buff) {
                1.0
            } else {
                0.0
            }),
            "is.algethar_puzzle_box" => Ok(if item.item_id == ALGETHAR_PUZZLE_BOX {
                1.0
            } else {
                0.0
            }),
            "cooldown.remains" | "cooldown.ready" => self.cooldown(slot, item, field, native),
            _ => Err(format!("unknown literal trinket fact {name}")),
        }
    }

    fn cooldown(
        &self,
        slot: &str,
        item: &Slot,
        field: &str,
        native: &NativeFacts<'_>,
    ) -> Result<f64, String> {
        let Some(_) = item.on_use else {
            // This pinned passive Guillotine has ProcCategoryRecovery0. SimC
            // unique_gear.cpp3767–3793 returns0 for its empty cooldown expressions.
            if item.item_id == 270173 {
                return Ok(0.0);
            }
            return Err(format!(
                "literal passive item{} proc cooldown unqualified",
                item.item_id
            ));
        };
        let property = field.strip_prefix("cooldown.").expect("validated above");
        native.value(&format!("cooldown.{slot}.{property}"))
    }
}

fn read_slot(world: &World, actor: Entity, slot: EquipmentSlot) -> Result<Slot, String> {
    let equipped = world
        .get::<EquippedItems>(actor)
        .ok_or("native equipped items absent")?;
    let &(_, source, item_id) = equipped
        .items
        .iter()
        .find(|(found, _, _)| *found == slot)
        .ok_or_else(|| format!("native equipped {slot:?} absent"))?;
    let template = world
        .resource::<ItemTemplates>()
        .get(source, item_id)
        .ok_or_else(|| format!("native item{item_id} definition absent"))?;
    let on_use = template
        .on_use
        .map(|on_use| -> Result<ItemUse, String> {
            Ok(ItemUse {
                item_id,
                on_use,
                item_level: read_item_level(
                    world,
                    actor,
                    slot,
                    &template.scaling,
                    template.inventory_type,
                )?,
                stat_buff: has_stat_aura(world, on_use.spell_id)?,
            })
        })
        .transpose()?;
    Ok(Slot { item_id, on_use })
}

fn read_item_level(
    world: &World,
    actor: Entity,
    slot: EquipmentSlot,
    scaling: &shared::item_scaling::ItemScaling,
    inventory_type: u8,
) -> Result<u16, String> {
    let modifiers = world
        .get::<EquippedItemModifiers>(actor)
        .ok_or("native instance modifiers absent")?;
    let (_, properties) = modifiers
        .items
        .iter()
        .find(|(found, _)| *found == slot)
        .ok_or_else(|| format!("native instance modifiers for {slot:?} absent"))?;
    let native = world.resource::<NativeItemInstances>();
    let modified = server::item_instance::bonuses::apply_item_bonuses(
        scaling,
        &properties.bonus_list,
        &native.bonuses,
    )?;
    Ok(server::item_instance::item_instance_stats(
        &modified.scaling,
        inventory_type,
        properties,
        world.resource::<crate::item_stats::ItemScalingTables>(),
    )?
    .item_level)
}

fn has_stat_aura(world: &World, spell: u32) -> Result<bool, String> {
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .ok_or_else(|| format!("native item spell{spell} absent"))?;
    Ok(spell
        .effects
        .iter()
        .any(|effect| matches!(effect.aura, 29 | 80 | 137 | 189)))
}
