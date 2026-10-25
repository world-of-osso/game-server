//! SimC76f8118 sc_shaman.cpp3039–3055/5850–5886/11919–11988:
//! weapon setup owns the native imbue buff; landed eligible melee drives its payload.
//! TC a352b1fa spell_shaman.cpp1263–1316/3302–3405 identifies enchant and proc spells.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Level};
use shared::protocol::EquipmentSlot;

use crate::auras::procs::{self, AuraEvent};
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{self, AuraInstance};
use crate::spell_cast;
use crate::spell_info::{SpellInfo, SpellStore};
use crate::stats::{EquippedItems, NativeItemInstances};

const WINDFURY: u32 = 319773;
const FLAMETONGUE: u32 = 319778;
const WINDFURY_ATTACK: u32 = 25504;
const FLAMETONGUE_ATTACK: u32 = 10444;
const UNRULY_WINDS: u32 = 384355;
const REQUIRES_MAIN_HAND: u32 = 0x0000_0400;
const REQUIRES_OFF_HAND: u32 = 0x0100_0000;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        WINDFURY,
        AuraScript {
            check_proc: Some(check_windfury),
            on_proc: Some(cast_windfury),
            ..Default::default()
        },
    );
    scripts.0.insert(
        FLAMETONGUE,
        AuraScript {
            check_proc: Some(check_flametongue),
            on_proc: Some(cast_flametongue),
            ..Default::default()
        },
    );
}

pub(super) fn apply(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let enhancement = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 263);
    let (enchant_spell, slot) = match spell.id {
        33757 => (334302, EquipmentSlot::MainHand),
        318038 if enhancement => (334294, EquipmentSlot::OffHand),
        318038 => (334294, EquipmentSlot::MainHand),
        _ => unreachable!("registered weapon imbue"),
    };
    if !has_equipped_weapon(world, caster, slot) {
        return;
    }
    apply_enchant_aura(world, caster, enchant_spell);
}

fn has_equipped_weapon(world: &World, caster: Entity, slot: EquipmentSlot) -> bool {
    let Some(equipped) = world.get::<EquippedItems>(caster) else {
        return false;
    };
    let templates = world.resource::<crate::item_templates::ItemTemplates>();
    equipped.items.iter().any(|&(key, source, item)| {
        key == slot
            && templates
                .get(source, item)
                .is_some_and(|template| matches!(template.inventory_type, 13 | 17 | 21 | 22))
    })
}

fn apply_enchant_aura(world: &mut World, caster: Entity, enchant_spell: u32) {
    let enchant = world
        .resource::<SpellStore>()
        .get(enchant_spell)
        .expect("native weapon enchant spell")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native enchant effect0")
        .misc_values[0] as u32;
    let level = world.get::<Level>(caster).expect("weapon owner level").0;
    let tables = &world.resource::<NativeItemInstances>().enchants;
    let payload = server::item_instance::enchants::enchantment_stats(enchant, level, tables)
        .unwrap_or_else(|error| panic!("native weapon enchant {enchant}: {error}"));
    for child in payload.equip_spells {
        spell_cast::cast_triggered(world, caster, child, Some(caster), 1);
    }
}

fn eligible(world: &World, aura: &AuraInstance, event: &AuraEvent, main_only: bool) -> bool {
    if event.damage <= 0.0 || event.periodic || auras::has_aura(world, aura.caster, 2645) {
        return false;
    }
    if event.spell_id.is_none() {
        return !main_only || event.actor_mask[0] & procs::PROC_MAIN_HAND_WEAPON_SWING != 0;
    }
    let Some(spell) = event
        .spell_id
        .and_then(|id| world.resource::<SpellStore>().get(id))
    else {
        return false;
    };
    let melee = spell
        .categories
        .as_ref()
        .is_some_and(|categories| categories.defense_type == 2);
    let mask = if main_only {
        REQUIRES_MAIN_HAND
    } else {
        REQUIRES_MAIN_HAND | REQUIRES_OFF_HAND
    };
    melee && spell_cast::attributes(spell, 3) & mask != 0
}

fn check_windfury(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    // SimC4467: Windfury cannot recursively trigger itself.
    event.spell_id != Some(WINDFURY_ATTACK) && eligible(world, aura, event, true)
}

fn check_flametongue(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    eligible(world, aura, event, false)
}

fn cast_windfury(world: &mut World, caster: Entity, _aura: &AuraInstance, target: Option<Entity>) {
    let Some(target) = target else {
        return;
    };
    // SimC11946–11958: two real payload attacks plus the selected native extra-attack roll.
    let extra = auras::has_aura(world, caster, UNRULY_WINDS)
        && crate::random::random::<f32>() * 100.0
            < super::script_effect_value(world, caster, UNRULY_WINDS, 0);
    for _ in 0..2 + u8::from(extra) {
        spell_cast::cast_triggered(world, caster, WINDFURY_ATTACK, Some(target), 1);
    }
}

fn cast_flametongue(
    world: &mut World,
    caster: Entity,
    _aura: &AuraInstance,
    target: Option<Entity>,
) {
    if let Some(target) = target {
        spell_cast::cast_triggered(world, caster, FLAMETONGUE_ATTACK, Some(target), 1);
    }
}
