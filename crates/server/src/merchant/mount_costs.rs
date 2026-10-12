//! Authenticated mount-only ItemExtendedCost costs; see mount_vendor_costs.provenance.json.
use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension};
use shared::currency::CurrencyWallet;
use shared::item_data::ItemDefinitionSource;
use shared::protocol::VendorCost;
use shared::reputation::CharacterReputation;

use super::*;

pub(super) const MOUNT_ITEMS: &str = "SELECT DISTINCT x.ItemID FROM item_x_item_effect x \
    JOIN item_effect e ON e.ID=x.ItemEffectID AND e.TriggerType=6 \
    JOIN mount m ON m.SourceSpellID=e.SpellID";

#[derive(Debug, Clone)]
pub(super) struct MountCost {
    pub money: u32,
    pub extra: Vec<VendorCost>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ReputationRequirement {
    faction: u32,
    rank: u8,
}

impl ReputationRequirement {
    pub fn met(self, reputation: Option<&CharacterReputation>) -> bool {
        reputation.is_some_and(|rep| rep.get_standing(self.faction) as u8 >= self.rank)
    }
}

pub(super) fn load_reputation(
    conn: &Connection,
) -> rusqlite::Result<HashMap<u32, ReputationRequirement>> {
    let mut statement = conn.prepare(&format!(
        "SELECT ID, MinFactionID, MinReputation FROM item_sparse \
         WHERE MinFactionID<>0 AND ID IN ({MOUNT_ITEMS})"
    ))?;
    statement
        .query_map([], |row| {
            let rank: u8 = row.get(2)?;
            assert!(rank <= 7, "unsupported mount reputation rank {rank}");
            Ok((
                row.get(0)?,
                ReputationRequirement {
                    faction: row.get(1)?,
                    rank,
                },
            ))
        })?
        .collect()
}

pub(super) fn load_costs(conn: &Connection) -> rusqlite::Result<HashMap<u32, MountCost>> {
    let mut costs = HashMap::new();
    let mut unavailable = HashSet::new();
    for line in include_str!("mount_vendor_costs.csv").lines().skip(1) {
        let values: Vec<u32> = line
            .split(',')
            .map(|v| v.parse().expect("pinned cost integer"))
            .collect();
        assert_eq!(values.len(), 22, "pinned cost column count");
        let mut cost = MountCost {
            money: values[1],
            extra: Vec::new(),
        };
        let mut supported = true;
        for (index, pair) in values[2..].chunks_exact(2).enumerate() {
            if pair[0] == 0 {
                continue;
            }
            let currency = index >= 5;
            match load_turnin(conn, currency, pair[0], pair[1])? {
                Some(turnin) => cost.extra.push(turnin),
                None => {
                    supported = false;
                    if unavailable.insert((currency, pair[0])) {
                        warn!(
                            currency,
                            id = pair[0],
                            "mount cost metadata unavailable; affected offers omitted"
                        );
                    }
                }
            }
        }
        if supported {
            costs.insert(values[0], cost);
        }
    }
    Ok(costs)
}

fn load_turnin(
    conn: &Connection,
    currency: bool,
    id: u32,
    raw: u32,
) -> rusqlite::Result<Option<VendorCost>> {
    let metadata: Option<(String, u32, u32)> = if currency {
        conn.query_row(
            "SELECT Name_lang, InventoryIconFileID, Flags_0 FROM currency_types WHERE ID=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
    } else {
        conn.query_row("SELECT c.Name, i.IconFileDataID, 0 FROM content_item c JOIN item i ON i.ID=c.ID WHERE c.ID=?1", [id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?
    };
    let Some((name, icon_file_data_id, flags)) = metadata else {
        return Ok(None);
    };
    // TrinityCore CurrencyTypesFlags::_100_Scaler (DBCEnums.h:863).
    let precision = if currency && flags & 0x8 != 0 { 100 } else { 1 };
    assert_eq!(raw % precision, 0, "fractional mount currency cost {id}");
    Ok(Some(VendorCost {
        currency,
        id,
        amount: raw / precision,
        name,
        icon_file_data_id,
    }))
}

/// Work only on detached copies until the complete purchase can commit.
pub(super) fn debit_turnins(
    shop: &Shop,
    player: Entity,
    model: &mut InventoryModel,
    cost: &MountCost,
    count: u32,
) -> Result<Option<CurrencyWallet>, MerchantError> {
    let mut wallet = if cost.extra.iter().any(|cost| cost.currency) {
        Some(
            shop.currencies
                .get(player)
                .map_err(|_| MerchantError::MissingTurnins)?
                .0
                .clone(),
        )
    } else {
        None
    };
    for turnin in &cost.extra {
        let amount = turnin
            .amount
            .checked_mul(count)
            .ok_or(MerchantError::MissingTurnins)?;
        if turnin.currency {
            let entry = wallet
                .as_mut()
                .expect("currency payment wallet")
                .entries
                .iter_mut()
                .find(|entry| entry.id == turnin.id)
                .ok_or(MerchantError::MissingTurnins)?;
            entry.amount = entry
                .amount
                .checked_sub(amount)
                .ok_or(MerchantError::MissingTurnins)?;
        } else {
            model
                .destroy_bag_items(ItemDefinitionSource::Retail, turnin.id, amount)
                .map_err(|_| MerchantError::MissingTurnins)?;
        }
    }
    Ok(wallet)
}

/// Gold, token destruction, delivery and currency share the same durable boundary.
pub(super) fn persist_purchase(
    shop: &mut Shop,
    customer: &Customer,
    model: InventoryModel,
    wallet: Option<CurrencyWallet>,
) {
    let gold = model.gold();
    let txn = shop.db().begin_write().expect("begin mount purchase");
    persistence::inventory_store::write_inventory_record(
        &txn,
        customer.character_id,
        &model.into_record(),
    );
    if let Some(wallet) = &wallet {
        let bytes = serde_json::to_vec(wallet).expect("encode mount payment wallet");
        txn.open_table(persistence::CURRENCIES)
            .expect("open mount payment currencies")
            .insert(customer.character_id, bytes.as_slice())
            .expect("write mount payment currencies");
    }
    txn.commit().expect("commit mount payment and delivery");
    shop.gold.record(customer.character_id, gold);
    if let Some(wallet) = wallet {
        shop.currencies
            .get_mut(customer.player)
            .expect("validated payment wallet")
            .0 = wallet.clone();
        crate::currency::send_currency_update(
            customer.connection,
            &wallet,
            None,
            None,
            &mut shop.currency_updates,
        );
    }
}
