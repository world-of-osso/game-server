//! Native follower consumables selected by the spawn loadout.
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use serde::Deserialize;
use shared::protocol::EquipmentSlot;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Debug, Default)]
pub struct LoadoutConsumables {
    flask: Option<u32>,
    food: Option<u32>,
    augment: Option<u32>,
    potion: Option<(u32, String)>,
    temporary_enchant: Vec<(EquipmentSlot, u32, Option<String>)>,
    wcl_potion: bool,
}

#[derive(Deserialize)]
struct WclSelection {
    spec: u32,
    flask: Option<u32>,
    food: Option<u32>,
    rune: Option<u32>,
    oil: Option<u32>,
    potion: Option<u32>,
    main_hand: bool,
    off_hand: bool,
}

/// The population, threshold and tie-break are documented alongside the WCL citations.
pub fn wcl(spec: u32) -> Result<LoadoutConsumables, String> {
    static SELECTIONS: OnceLock<Result<Vec<WclSelection>, String>> = OnceLock::new();
    let rows = SELECTIONS
        .get_or_init(|| {
            serde_json::from_str(include_str!("wcl_consumables.json"))
                .map_err(|error| format!("invalid WCL consumable selections: {error}"))
        })
        .as_ref()
        .map_err(Clone::clone)?;
    let row = rows
        .iter()
        .find(|row| row.spec == spec)
        .ok_or_else(|| format!("WCL consumables unavailable for spec{spec}"))?;
    let mut oils = Vec::new();
    if let Some(enchant) = row.oil {
        if row.main_hand {
            oils.push((EquipmentSlot::MainHand, enchant, None));
        }
        if row.off_hand {
            oils.push((EquipmentSlot::OffHand, enchant, None));
        }
    }
    Ok(LoadoutConsumables {
        flask: row.flask.map(flask_item).transpose()?,
        food: row.food,
        augment: row.rune.map(augment_item).transpose()?,
        potion: row.potion.map(wcl_potion).transpose()?,
        temporary_enchant: oils,
        wcl_potion: row.potion.is_some(),
    })
}

fn flask_item(spell: u32) -> Result<u32, String> {
    match spell {
        1235057 => Ok(241320),
        1235108 => Ok(241322),
        1235110 => Ok(241324),
        1235111 => Ok(241326),
        _ => Err(format!("unsupported WCL flask spell{spell}")),
    }
}

fn augment_item(spell: u32) -> Result<u32, String> {
    match spell {
        1264426 => Ok(259085),
        1295329 => Ok(274797),
        _ => Err(format!("unsupported WCL augment spell{spell}")),
    }
}

fn wcl_potion(spell: u32) -> Result<(u32, String), String> {
    let (item, name) = match spell {
        1236616 => (241308, "lights_potential_2"),
        1236994 => (241288, "potion_of_recklessness_2"),
        1236998 => (241292, "draught_of_rampant_abandon_2"),
        1295132 => (271887, "liquid_luster_2"),
        _ => return Err(format!("unsupported WCL potion spell{spell}")),
    };
    Ok((item, name.into()))
}

/// Supported SimC potion tokens and their native item identities.
pub const SIMC_POTIONS: &[(&str, u32)] = &[
    ("lights_potential_2", 241308),
    ("lights_potential_1", 241309),
    ("potion_of_recklessness_2", 241288),
    ("potion_of_recklessness_1", 241289),
    ("draught_of_rampant_abandon_2", 241292),
    ("draught_of_rampant_abandon_1", 241293),
    ("potion_of_devoured_dreams_2", 241294),
    ("potion_of_devoured_dreams_1", 241295),
    ("potion_of_zealotry_2", 241296),
    ("potion_of_zealotry_1", 241297),
    ("liquid_luster_2", 271887),
    ("liquid_luster_1", 271886),
];

/// Resolve the exact configured SimC identities; unsupported tokens fail explicitly.
pub fn simc(values: &BTreeMap<String, String>) -> Result<LoadoutConsumables, String> {
    let mut result = LoadoutConsumables::default();
    for (kind, value) in values {
        if value.is_empty() || value == "disabled" {
            continue;
        }
        match kind.as_str() {
            "flask" => {
                result.flask = Some(match value.as_str() {
                    "flask_of_the_magisters_2" | "magisters_2" => 241322,
                    "flask_of_the_magisters_1" | "magisters_1" => 241323,
                    "flask_of_the_blood_knights_2" => 241324,
                    "flask_of_the_blood_knights_1" => 241325,
                    "flask_of_the_shattered_sun_2" => 241326,
                    "flask_of_the_shattered_sun_1" => 241327,
                    "flask_of_thalassian_resistance_2" => 241320,
                    "flask_of_thalassian_resistance_1" => 241321,
                    _ => return Err(format!("unsupported SimC flask {value:?}")),
                })
            }
            "food" => {
                result.food = Some(match value.as_str() {
                    "royal_roast" => 1294727,
                    "harandar_celebration" | "silvermoon_parade" => 1232585,
                    "blooming_feast" => 1232091,
                    _ => return Err(format!("unsupported SimC food {value:?}")),
                })
            }
            "augmentation" => {
                result.augment = Some(match value.as_str() {
                    "void_touched" | "void_touched_augment_rune" => 259085,
                    "tidesworn" | "tidesworn_augment_rune" => 274797,
                    _ => return Err(format!("unsupported SimC augmentation {value:?}")),
                })
            }
            "potion" => {
                let item = SIMC_POTIONS
                    .iter()
                    .find_map(|&(name, item)| (name == value).then_some(item))
                    .ok_or_else(|| format!("unsupported SimC potion {value:?}"))?;
                result.potion = Some((item, value.clone()));
            }
            "temporary_enchant" => result.temporary_enchant = parse_enchants(value)?,
            _ => return Err(format!("unsupported SimC consumable {kind:?}")),
        }
    }
    Ok(result)
}

fn parse_enchants(value: &str) -> Result<Vec<(EquipmentSlot, u32, Option<String>)>, String> {
    value
        .split('/')
        .map(|entry| {
            let (value, option) = entry
                .split_once(',')
                .map_or((entry, None), |(v, o)| (v, Some(o)));
            let (slot, name) = value
                .split_once(':')
                .ok_or("temporary enchant lacks slot")?;
            let slot = super::parity_profile::slot(slot)?;
            let enchant = match name {
                "thalassian_phoenix_oil_1" => 8051,
                "thalassian_phoenix_oil_2" => 8052,
                "oil_of_dawn_1" => 8053,
                "oil_of_dawn_2" => 8054,
                "refulgent_whetstone_1" => 7906,
                "refulgent_whetstone_2" => 7905,
                "refulgent_weightstone_1" => 7907,
                "refulgent_weightstone_2" => 7908,
                _ => return Err(format!("unsupported SimC temporary enchant {name:?}")),
            };
            let condition = option
                .map(|option| {
                    option
                        .strip_prefix("if=")
                        .map(str::to_owned)
                        .ok_or_else(|| format!("unsupported temporary enchant option {option:?}"))
                })
                .transpose()?;
            Ok((slot, enchant, condition))
        })
        .collect()
}

fn condition_active(world: &World, actor: Entity, condition: &str) -> Result<bool, String> {
    use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
    let expression = crate::simc_apl::Expression::parse(condition)?;
    let bindings = Bindings {
        talents: [
            ("flametongue_weapon".into(), 318038),
            ("rite_of_adjuration".into(), 433583),
            ("rite_of_sanctification".into(), 433568),
        ]
        .into(),
        ..Default::default()
    };
    let facts = NativeFacts::new(
        world,
        actor,
        &bindings,
        Encounter::dungeon(crate::spell_cast::now(world)),
        0,
    );
    Ok(expression.evaluate(&facts)? != 0.0)
}

fn install_enchants(
    world: &mut World,
    actor: Entity,
    selections: &[(EquipmentSlot, u32, Option<String>)],
) -> Result<(), String> {
    world
        .get_mut::<crate::stats::EquippedItemModifiers>(actor)
        .ok_or("consumable item modifiers unavailable")?
        .temporary_enchants
        .retain(|(_, enchant)| !matches!(enchant, 6188 | 7905..=7908 | 8051..=8055));
    for (slot, enchant, condition) in selections {
        if !world
            .get::<crate::stats::EquippedItems>(actor)
            .ok_or("consumable equipment unavailable")?
            .items
            .iter()
            .any(|(equipped, _, _)| equipped == slot)
        {
            continue;
        }
        if let Some(condition) = condition
            && !condition_active(world, actor, condition)?
        {
            continue;
        }
        let mut modifiers = world
            .get_mut::<crate::stats::EquippedItemModifiers>(actor)
            .ok_or("consumable item modifiers unavailable")?;
        modifiers
            .temporary_enchants
            .retain(|(existing, _)| existing != slot);
        modifiers.temporary_enchants.push((*slot, *enchant));
    }
    Ok(())
}

fn apply_item(world: &mut World, actor: Entity, id: u32) -> Result<(), String> {
    let item = crate::consumables::ItemUse::consumable(world, id)?;
    crate::consumables::apply(world, actor, item.on_use.spell_id, Some(item.item_level))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{EquippedItemModifiers, EquippedItems};

    #[test]
    fn wcl_snapshot_oil_does_not_override_spec_policy() {
        let items = super::super::gear::wcl::select_instances(65, 6).unwrap();
        assert!(
            items
                .modifiers
                .temporary_enchants
                .contains(&(EquipmentSlot::MainHand, 8052))
        );
        let mut modifiers = items.modifiers;
        modifiers
            .temporary_enchants
            .push((EquipmentSlot::OffHand, 5401));
        let mut world = World::new();
        let actor = world
            .spawn((EquippedItems { items: items.items }, modifiers))
            .id();

        install_enchants(&mut world, actor, &wcl(65).unwrap().temporary_enchant).unwrap();
        assert_eq!(
            world
                .get::<EquippedItemModifiers>(actor)
                .unwrap()
                .temporary_enchants,
            vec![(EquipmentSlot::OffHand, 5401)]
        );
    }
}

/// Install the selected native auras, weapon enchants and potion before spawn completes.
pub fn install(
    world: &mut World,
    actor: Entity,
    selection: &LoadoutConsumables,
) -> Result<(), String> {
    install_enchants(world, actor, &selection.temporary_enchant)?;
    if let Some(item) = selection.flask {
        apply_item(world, actor, item)?;
    }
    if let Some(item) = selection.augment {
        apply_item(world, actor, item)?;
    }
    if let Some(spell) = selection.food {
        if spell == 1232091 {
            // Blooming Feast selects from the composed native ratings.
            world
                .run_system_once(crate::stats::update_character_stats)
                .map_err(|error| error.to_string())?;
        }
        crate::consumables::apply(world, actor, spell, None)?;
    }
    world.entity_mut(actor).remove::<(
        crate::consumables::Potion,
        crate::consumables::WclPotion,
        crate::simc_apl_native::ConfiguredPotion,
    )>();
    if let Some((id, name)) = &selection.potion {
        let item = crate::consumables::ItemUse::consumable(world, *id)?;
        world.entity_mut(actor).insert((
            crate::consumables::Potion(item),
            crate::simc_apl_native::ConfiguredPotion {
                name: name.clone(),
                on_use: item.on_use,
            },
        ));
        if selection.wcl_potion {
            world
                .entity_mut(actor)
                .insert(crate::consumables::WclPotion);
        }
    }
    Ok(())
}
