//! Profile effects resolve native catalogs, never reference-report stat or damage amounts.
use super::inputs::Profile;
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};

#[derive(Component, Clone, Copy)]
pub(super) struct Potion(pub super::literal_rotation::equipment::ItemUse);

fn database() -> Result<Connection, String> {
    let path = std::env::var_os("DPS_PARITY_WORLD_DB").ok_or("native effect database absent")?;
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())
}

pub(super) fn install(world: &mut World, actor: Entity, profile: &Profile) -> Result<(), String> {
    let db = database()?;
    install_expansion(world, actor, profile, &db)?;
    for (kind, value) in &profile.consumables {
        if value == "disabled" || value.is_empty() {
            continue;
        }
        // SimC76f8118 generated consumable/temporary_enchant bindings supply identities only.
        // Native ItemSparse/ItemEffect/SpellItemEnchantment supply all values and quality scaling.
        match (kind.as_str(), value.as_str()) {
            ("flask", "flask_of_the_magisters_2" | "magisters_2") => {
                apply_item(world, actor, 241322, &db)?
            }
            // SimC76f8118 consumable.cpp40–59/item_data.inc175834: quality2 identity.
            ("flask", "flask_of_the_blood_knights_2") => apply_item(world, actor, 241324, &db)?,
            // SimC76f8118 item_data.inc175830: quality2 native item/ilvl295.
            ("flask", "flask_of_thalassian_resistance_2") => apply_item(world, actor, 241320, &db)?,
            // consumable.cpp613 accepts both short and full Void-Touched tokens.
            ("augmentation", "void_touched" | "void_touched_augment_rune") => {
                apply_item(world, actor, 259085, &db)?
            }
            // Food selectors require the composed native equipment/consumable ratings.
            ("food", _) => {}
            ("potion", _) => {
                let Some(id) = potion_item(value) else {
                    continue;
                };
                let item = super::literal_rotation::equipment::ItemUse::consumable(world, id)?;
                world.entity_mut(actor).insert((
                    Potion(item),
                    crate::simc_apl_native::ConfiguredPotion {
                        name: value.clone(),
                        on_use: item.on_use,
                    },
                ));
            }
            ("temporary_enchant", _) => super::profile_oils::install(world, actor, value)?,
            _ => {} // Report retains every unsupported token explicitly.
        }
    }
    Ok(())
}

pub(super) fn install_food(
    world: &mut World,
    actor: Entity,
    profile: &Profile,
) -> Result<(), String> {
    let Some(food) = profile.consumables.get("food") else {
        return Ok(());
    };
    // SimC76f8118 unique_gear_midnight.cpp6388/6395–6398: native food identities.
    let spell = match food.as_str() {
        "royal_roast" => 1294727,
        "harandar_celebration" | "silvermoon_parade" => 1232585,
        "blooming_feast" => 1232091,
        _ => return Ok(()), // Disabled/unsupported tokens retain their report semantics.
    };
    server::consumables::apply(world, actor, spell, None)
}

fn item_spell(db: &Connection, item: u32) -> Result<(u32, u16), String> {
    db.query_row("SELECT e.SpellID,s.ItemLevel FROM item_sparse s JOIN item_x_item_effect x ON x.ItemID=s.ID JOIN item_effect e ON e.ID=x.ItemEffectID WHERE s.ID=?1 AND e.TriggerType=0", [item], |r| Ok((r.get(0)?,r.get(1)?)))
        .map_err(|e| format!("native consumable item{item}: {e}"))
}
fn apply_item(world: &mut World, actor: Entity, item: u32, db: &Connection) -> Result<(), String> {
    let (spell, level) = item_spell(db, item)?;
    server::consumables::apply(world, actor, spell, Some(level))
}

fn install_expansion(
    world: &mut World,
    actor: Entity,
    profile: &Profile,
    db: &Connection,
) -> Result<(), String> {
    let Some(entries) = &profile.omnium_talents else {
        return Ok(());
    };
    // unique_gear.cpp3677–3705: numeric tokens are TraitNodeEntry IDs, not nodes or class talents.
    for entry in entries.split('/') {
        let (entry, _) = entry.split_once(':').unwrap_or((entry, "1"));
        let entry = resolve_expansion_entry(entry)?;
        let spell: u32 = db.query_row("SELECT d.SpellID FROM trait_node_entry e JOIN trait_definition d ON d.ID=e.TraitDefinitionID WHERE e.ID=?1",[entry],|r|r.get(0)).map_err(|e| format!("native expansion entry{entry}: {e}"))?;
        server::consumables::apply(world, actor, spell, None)?;
    }
    Ok(())
}

// SimC76f8118 unique_gear.cpp3681–3698 resolves tokenized expansion names or
// numeric TraitNodeEntry IDs. These identities are native70448 Omnium tree1186.
const NAMED_EXPANSION_ENTRIES: &[(&str, u32)] = &[
    ("rune_of_unleashed_fire", 136822),
    ("rune_of_selfmending", 136819),
    ("rune_of_lingering", 136817),
    ("rune_of_critical_power", 136815),
    ("rune_of_overload", 136814),
    ("rune_of_lynxlike_reflexes", 136823),
    ("rune_of_burning_haste", 136821),
];

fn resolve_expansion_entry(token: &str) -> Result<u32, String> {
    if let Some((_, entry)) = NAMED_EXPANSION_ENTRIES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(token))
    {
        return Ok(*entry);
    }
    token
        .parse()
        .map_err(|error| format!("native expansion entry {token:?}: {error}"))
}

fn potion_item(token: &str) -> Option<u32> {
    // SimC76f8118 item_data.inc175810/175828: identities only; native catalogs
    // supply each potion's spell, item level, coefficient and cooldown.
    match token {
        "potion_of_recklessness_2" => Some(241288),
        "lights_potential_2" => Some(241308),
        // SimC76f8118 item_data.inc182728: Liquid Luster quality2, native ilvl295.
        "liquid_luster_2" => Some(271887),
        _ => None,
    }
}

pub(super) fn supported(kind: &str, value: &str) -> bool {
    if value == "disabled" || value.is_empty() {
        return true;
    }
    matches!(
        (kind, value),
        ("flask", "flask_of_the_magisters_2" | "magisters_2")
            | ("flask", "flask_of_the_blood_knights_2")
            | ("flask", "flask_of_thalassian_resistance_2")
            | ("augmentation", "void_touched" | "void_touched_augment_rune")
            | ("food", "royal_roast")
            | (
                "food",
                "harandar_celebration" | "silvermoon_parade" | "blooming_feast"
            )
    ) || (kind == "potion" && potion_item(value).is_some())
        || (kind == "temporary_enchant" && super::profile_oils::supported(value))
}
