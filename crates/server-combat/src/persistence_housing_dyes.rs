//! Atomic committed dye costs. Preview is read-only; clearing never refunds spent items.
use super::*;
use crate::housing_catalog::DyeCatalog;
use shared::protocol::{DecorDye, DecorDyePreview};

pub(crate) fn preview_dyes(
    db: &Database,
    actor: Actor,
    house_id: u64,
    instance_id: u64,
    changes: &[DecorDye],
    catalog: &DyeCatalog,
) -> Result<DecorDyePreview, String> {
    let txn = db.begin_read().map_err(|error| error.to_string())?;
    let state = read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)?;
    require_house_editor(&state, actor, house_id)?;
    let decor = state.houses[&house_id]
        .decor
        .iter()
        .find(|decor| decor.instance_id == instance_id)
        .ok_or("Placed decor not found in this house.")?;
    catalog.costs(decor.decor_id, &decor.dyes, changes)?;
    Ok(DecorDyePreview {
        house_id,
        instance_id,
        changes: changes.to_vec(),
    })
}

pub(crate) fn commit_dyes(
    db: &Database,
    actor: Actor,
    preview: &DecorDyePreview,
    catalog: &DyeCatalog,
) -> Result<bool, String> {
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    let mut state = read_in(&txn)?;
    require_house_editor(&state, actor, preview.house_id)?;
    let selected = find_decor(&mut state, preview.house_id, preview.instance_id)?.clone();
    let costs = catalog.costs(selected.decor_id, &selected.dyes, &preview.changes)?;
    let dyes = apply_dye_changes(&selected.dyes, &preview.changes);
    if dyes == selected.dyes {
        return Ok(false);
    }
    let mut inventory = read_inventory(&txn, actor.character_id)?;
    consume_dyes(&txn, actor.character_id, &mut inventory, &costs)?;
    find_decor(&mut state, preview.house_id, preview.instance_id)?.dyes = dyes;
    write_inventory(&txn, actor.character_id, &inventory)?;
    commit(txn, &state)?;
    Ok(true)
}

fn apply_dye_changes(current: &[DecorDye], changes: &[DecorDye]) -> Vec<DecorDye> {
    let mut dyes = current.to_vec();
    for change in changes {
        dyes.retain(|saved| saved.slot_id != change.slot_id);
        if change.color_id.is_some() {
            dyes.push(change.clone());
        }
    }
    dyes.sort_by_key(|dye| dye.slot_id);
    dyes
}

fn consume_dyes(
    txn: &WriteTransaction,
    character: u64,
    inventory: &mut InventoryData,
    costs: &BTreeMap<u32, u32>,
) -> Result<(), String> {
    for (&item_id, &amount) in costs {
        consume_owned_dye(txn, character, inventory, item_id, amount)?;
    }
    Ok(())
}

fn consume_owned_dye(
    txn: &WriteTransaction,
    character: u64,
    inventory: &mut InventoryData,
    item_id: u32,
    mut remaining: u32,
) -> Result<(), String> {
    let sources = txn
        .open_table(inventory_store::ITEM_DEFINITION_SOURCES)
        .map_err(|error| error.to_string())?;
    let mut depleted = std::collections::BTreeSet::new();
    for item in &mut inventory.items {
        if item.item_id != item_id
            || inventory_store::read_item_definition_source(&sources, item.item_guid)
                != ItemDefinitionSource::Retail
        {
            continue;
        }
        let used = remaining.min(item.stack_count);
        item.stack_count -= used;
        remaining -= used;
        if used > 0 && item.stack_count == 0 {
            depleted.insert(item.item_guid);
        }
        if remaining == 0 {
            break;
        }
    }
    if remaining != 0 {
        return Err(format!("Not enough owned Retail dye item {item_id}."));
    }
    drop(sources);
    for &guid in &depleted {
        remove_placement(txn, character, guid)?;
    }
    inventory
        .items
        .retain(|item| !depleted.contains(&item.item_guid));
    Ok(())
}
