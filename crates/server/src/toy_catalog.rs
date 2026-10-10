//! Toy.db2 identities, joined to Retail item and spell tables in world.db.
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use rusqlite::Connection;
use shared::protocol::ToySnapshot;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub(crate) struct Toy {
    pub snapshot: ToySnapshot,
    pub item_cooldown_ms: i32,
    pub category: u32,
    pub category_cooldown_ms: i32,
    pub condition_id: u32,
}

#[derive(Resource, Default)]
pub(crate) struct ToyCatalog(pub BTreeMap<u32, Toy>);

pub(crate) fn load(conn: &Connection) -> rusqlite::Result<ToyCatalog> {
    let mut query = conn.prepare("SELECT t.ItemID, s.Display_lang, i.IconFileDataID, s.ExpansionID, t.Flags, t.SourceTypeEnum, t.SourceText_lang, e.SpellID, e.CoolDownMSec, e.SpellCategoryID, e.CategoryCoolDownMSec, e.PlayerConditionID, e.ChrSpecializationID FROM toy t LEFT JOIN item i ON i.ID=t.ItemID LEFT JOIN item_sparse s ON s.ID=t.ItemID LEFT JOIN (SELECT x.ItemID,e.*,ROW_NUMBER() OVER (PARTITION BY x.ItemID ORDER BY e.LegacySlotIndex,e.ID) AS slot FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID WHERE e.TriggerType=0 AND e.SpellID>0) e ON e.ItemID=t.ItemID AND e.slot=1 ORDER BY t.ItemID")?;
    let rows = query
        .query_map([], |r| {
            let name: Option<String> = r.get(1)?;
            let icon: Option<u32> = r.get(2)?;
            let spell_id: Option<u32> = r.get(7)?;
            let spec: Option<u32> = r.get(12)?;
            let unavailable_reason = if name.is_none() || icon.is_none() {
                Some("unresolved Item/ItemSparse row".into())
            } else if spell_id.is_none() {
                Some("unresolved ItemEffect on-use spell".into())
            } else if spec.is_some_and(|id| id != 0) {
                Some("toy specialization restriction not supported".into())
            } else {
                None
            };
            Ok(Toy {
                snapshot: ToySnapshot {
                    item_id: r.get(0)?,
                    name: name.unwrap_or_default(),
                    icon_file_data_id: icon.unwrap_or_default(),
                    expansion_id: r.get::<_, Option<i32>>(3)?.unwrap_or(0),
                    flags: r.get(4)?,
                    source_type: r.get(5)?,
                    source_text: r.get(6)?,
                    spell_id,
                    learned: false,
                    favourite: false,
                    unavailable_reason,
                },
                item_cooldown_ms: r.get::<_, Option<i32>>(8)?.unwrap_or(-1),
                category: r.get::<_, Option<u32>>(9)?.unwrap_or(0),
                category_cooldown_ms: r.get::<_, Option<i32>>(10)?.unwrap_or(-1),
                condition_id: r.get::<_, Option<u32>>(11)?.unwrap_or(0),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ToyCatalog(
        rows.into_iter()
            .map(|toy| (toy.snapshot.item_id, toy))
            .collect(),
    ))
}

/// Registry-backed census and preflight, including reachable triggered spells.
/// Script aura types without a toy script are not implemented merely by replication.
pub(crate) fn unsupported_types(store: &SpellStore, spell_id: u32) -> BTreeSet<String> {
    let mut reasons = BTreeSet::new();
    classify(store, spell_id, 0, &mut BTreeSet::new(), &mut reasons);
    reasons
}

fn classify(
    store: &SpellStore,
    spell_id: u32,
    depth: u8,
    visiting: &mut BTreeSet<u32>,
    reasons: &mut BTreeSet<String>,
) {
    if !visiting.insert(spell_id) {
        return;
    }
    let Some(spell) = store.get(spell_id) else {
        reasons.insert("missing spell".into());
        return;
    };
    if spell.effects.is_empty() {
        reasons.insert("missing effects".into());
    }
    for effect in &spell.effects {
        if effect.effect == 0 {
            continue;
        }
        if crate::auras::is_aura_effect(effect) {
            let kind = crate::auras::handlers::kind(effect.aura);
            if kind.is_none()
                || kind == Some(crate::auras::handlers::AuraKind::Script)
                || (effect.aura == crate::auras::handlers::TRANSFORM && effect.misc_values[0] == 0)
            {
                reasons.insert(format!("aura {}", effect.aura));
            }
        } else if !crate::spell_cast::effects::is_handled(effect) {
            reasons.insert(format!("effect {}", effect.effect));
        }
        if effect.trigger_spell != 0 {
            if depth >= crate::spell_cast::effects::MAX_TRIGGER_DEPTH {
                reasons.insert("trigger depth limit".into());
            } else {
                classify(store, effect.trigger_spell, depth + 1, visiting, reasons);
            }
        }
    }
    visiting.remove(&spell_id);
}

pub(crate) fn unavailable(toy: &Toy, store: &SpellStore) -> Option<String> {
    if let Some(reason) = &toy.snapshot.unavailable_reason {
        return Some(reason.clone());
    }
    if toy.condition_id != 0 {
        return Some(format!(
            "toy PlayerCondition {} not supported",
            toy.condition_id
        ));
    }
    let types = unsupported_types(store, toy.snapshot.spell_id?);
    (!types.is_empty()).then(|| {
        format!(
            "unsupported toy spell: {}",
            types.into_iter().collect::<Vec<_>>().join(", ")
        )
    })
}
