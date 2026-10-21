//! Source markers for summoned-toy interactions, independent of template residency.
//! Blank TDB script names are not evidence that a summoned actor is scenery only.
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

const MARKERS: &str = "WITH RECURSIVE roots AS (
    SELECT x.ItemID, e.SpellID, ROW_NUMBER() OVER (
        PARTITION BY x.ItemID ORDER BY e.LegacySlotIndex,e.ID) AS slot
    FROM toy t JOIN item_x_item_effect x ON x.ItemID=t.ItemID
    JOIN item_effect e ON e.ID=x.ItemEffectID
    WHERE e.TriggerType=0 AND e.SpellID>0
), reachable(item,spell) AS (
    SELECT ItemID,SpellID FROM roots WHERE slot=1
    UNION SELECT r.item,e.EffectTriggerSpell FROM reachable r
      JOIN spell_effect e ON e.SpellID=r.spell
      WHERE e.DifficultyID=0 AND e.EffectTriggerSpell>0
), summons(item,entry) AS (
    SELECT DISTINCT r.item,e.EffectMiscValue_0 FROM reachable r
      JOIN spell_effect e ON e.SpellID=r.spell
      WHERE e.DifficultyID=0 AND e.Effect=28
)
SELECT s.item,'summon creature '||s.entry||' npcflag '||t.npcflag
FROM summons s JOIN tdb_creature_template t ON t.entry=s.entry WHERE t.npcflag<>0
UNION SELECT s.item,'summon creature '||s.entry||' VehicleId '||t.VehicleId
FROM summons s JOIN tdb_creature_template t ON t.entry=s.entry WHERE t.VehicleId<>0
UNION SELECT s.item,'summon creature '||s.entry||' spellclick rows'
FROM summons s JOIN tdb_npc_spellclick_spells c ON c.npc_entry=s.entry
UNION SELECT s.item,'summon creature '||s.entry||' smart_scripts rows'
FROM summons s JOIN tdb_smart_scripts a ON a.entryorguid=s.entry AND a.source_type=0
UNION SELECT r.item,'gameobject effect '||e.Effect||' spell '||r.spell
FROM reachable r JOIN spell_effect e ON e.SpellID=r.spell
WHERE e.DifficultyID=0 AND e.Effect IN (50,76,86,104,105,106,107)
  AND EXISTS(SELECT 1 FROM summons s WHERE s.item=r.item)";

// Explicit existing toyfx interaction contracts. No behavior is invented for them.
const CONTRACT_GAPS: &[(u32, &str)] = &[
    (88387, "source spittoon interaction"),
    (88579, "source brew interaction"),
    (193478, "source seating interaction"),
    (198156, "source wormhole interaction"),
    (219387, "source fireworks interaction"),
    // Additional explicit toyfx11/toyfx18 source contracts, not item-name inference.
    (88375, "source target-dummy interaction"),
    (144339, "source target-dummy interaction"),
    (116689, "source seating interaction"),
    (172924, "source wormhole interaction"),
    (122126, "source weighted invisible-display contract"),
    // toyfx11 retains opt-in party/viewer world-effect contracts, not just actors.
    (198206, "source party-viewer world-effect interaction"),
    (198264, "source party-viewer world-effect interaction"),
    (210975, "source party-viewer world-effect interaction"),
];

pub(super) fn load(conn: &Connection) -> rusqlite::Result<BTreeMap<u32, BTreeSet<String>>> {
    let mut query = conn.prepare(MARKERS)?;
    let rows = query.query_map([], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut gaps: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    for row in rows {
        let (item, marker) = row?;
        gaps.entry(item).or_default().insert(marker);
    }
    for &(item, marker) in CONTRACT_GAPS {
        gaps.entry(item).or_default().insert(marker.into());
    }
    Ok(gaps)
}

fn is_proven_mailbox_marker(item: u32, spell: Option<u32>, marker: &str) -> bool {
    // Sole exemption: toyfx66_katy_real_learning_summon_mailbox_service_and_source_expiry.
    // Remove only this proved source flag marker; additional/drifted markers still refuse.
    item == 156833 && spell == Some(261602) && marker == "summon creature 132969 npcflag 67108865"
}

pub(super) fn unavailable(
    gaps: &BTreeMap<u32, BTreeSet<String>>,
    item: u32,
    spell: Option<u32>,
) -> Option<String> {
    let markers: Vec<_> = gaps
        .get(&item)?
        .iter()
        .filter(|marker| !is_proven_mailbox_marker(item, spell, marker))
        .map(String::as_str)
        .collect();
    (!markers.is_empty()).then(|| format!("unimplemented toy interaction: {}", markers.join(", ")))
}
