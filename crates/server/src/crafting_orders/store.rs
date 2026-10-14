use super::Order;
use crate::persistence::inventory_store::{InventoryRecord, write_inventory_record};
use crate::persistence::mail_store::{MailData, push_mail};
use crate::persistence::{INVENTORIES, InventoryData};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, TableError};

const ORDERS: TableDefinition<u64, &[u8]> = TableDefinition::new("crafting_orders_v1");

pub(crate) fn all(db: &Database) -> Result<Vec<Order>, String> {
    let txn = db.begin_read().map_err(|e| e.to_string())?;
    let table = match txn.open_table(ORDERS) {
        Ok(table) => table,
        Err(TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };
    table
        .iter()
        .map_err(|e| e.to_string())?
        .map(|row| {
            let (_, value) = row.map_err(|e| e.to_string())?;
            bitcode::decode(value.value()).map_err(|e| e.to_string())
        })
        .collect()
}

pub(crate) fn load(db: &Database, id: u64) -> Result<Order, String> {
    let txn = db.begin_read().map_err(|e| e.to_string())?;
    let table = txn
        .open_table(ORDERS)
        .map_err(|e| format!("load order {id}: {e}"))?;
    let row = table
        .get(id)
        .map_err(|e| e.to_string())?
        .ok_or("order not found")?;
    bitcode::decode(row.value()).map_err(|e| format!("decode order {id}: {e}"))
}

/// Order, inventories and mail commit together. Stale reads never publish changes.
pub(crate) fn commit(
    db: &Database,
    expected: Option<&Order>,
    order: &Order,
    records: &[(u64, InventoryData, InventoryRecord)],
    mails: &[(u64, MailData)],
) -> Result<(), String> {
    let txn = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = txn.open_table(ORDERS).map_err(|e| e.to_string())?;
        let current = table
            .get(order.id)
            .map_err(|e| e.to_string())?
            .map(|row| bitcode::decode::<Order>(row.value()))
            .transpose()
            .map_err(|e| e.to_string())?;
        if current.as_ref() != expected {
            return Err("order changed since it was read".into());
        }
        let inventories = txn.open_table(INVENTORIES).map_err(|e| e.to_string())?;
        for (id, before, _) in records {
            let current = inventories
                .get(*id)
                .map_err(|e| e.to_string())?
                .ok_or("inventory disappeared")?;
            if bitcode::decode::<InventoryData>(current.value()).map_err(|e| e.to_string())?
                != *before
            {
                return Err("inventory changed since it was read".into());
            }
        }
        table
            .insert(order.id, bitcode::encode(order).as_slice())
            .map_err(|e| e.to_string())?;
    }
    for (id, _, record) in records {
        write_inventory_record(&txn, *id, record);
    }
    for (id, mail) in mails {
        for attachment in &mail.items {
            crate::persistence::inventory_store::write_item_definition_source(
                &txn,
                attachment.item.item_guid,
                shared::item_data::ItemDefinitionSource::Retail,
            );
        }
        push_mail(&txn, *id, mail.clone());
    }
    crate::persistence_commit::commit(txn).map_err(|e| format!("commit crafting order: {e}"))
}
