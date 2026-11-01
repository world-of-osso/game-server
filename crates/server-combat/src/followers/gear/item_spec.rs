//! Native ItemSpec loot-specialization eligibility for follower weapons.
use crate::item_templates::ItemTemplate;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

struct Rule {
    max_level: u32,
    primary: u32,
    secondary: u32,
    spec: u32,
    class: u8,
}

pub(super) struct ItemSpecs {
    weapons: Vec<Rule>,
    overrides: HashMap<u32, HashSet<u32>>,
}

impl ItemSpecs {
    pub(super) fn load(db: &Connection) -> rusqlite::Result<Self> {
        let mut query = db.prepare("SELECT i.MaxLevel,i.PrimaryStat,i.SecondaryStat,i.SpecializationID,s.ClassID FROM item_spec i JOIN chr_specialization s ON s.ID=i.SpecializationID WHERE i.ItemType=5")?;
        let weapons = query
            .query_map([], |row| {
                Ok(Rule {
                    max_level: row.get(0)?,
                    primary: row.get(1)?,
                    secondary: row.get(2)?,
                    spec: row.get(3)?,
                    class: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut overrides: HashMap<u32, HashSet<u32>> = HashMap::new();
        let mut query = db.prepare("SELECT o.ItemID,o.SpecID FROM item_spec_override o JOIN chr_specialization s ON s.ID=o.SpecID")?;
        for row in query.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))? {
            let (item, spec) = row?;
            overrides.entry(item).or_default().insert(spec);
        }
        Ok(Self { weapons, overrides })
    }

    pub(super) fn allows(&self, id: u32, item: &ItemTemplate, spec: u32, level: u8) -> bool {
        if let Some(specs) = self.overrides.get(&id) {
            return specs.contains(&spec);
        }
        let mut stats = weapon_stats(item.scaling.subclass_id);
        for stat in &item.scaling.stats {
            match stat.stat {
                3 => stats.push(1),
                4 => stats.push(2),
                5 => stats.push(0),
                13 => stats.push(5),
                14 => stats.push(6),
                19..=21 | 32 => stats.push(24),
                36 => stats.push(25),
                31 => stats.push(4),
                50 => stats.push(26),
                71 => stats.extend([1, 2, 0]),
                72 => stats.extend([1, 2]),
                73 => stats.extend([1, 0]),
                74 => stats.extend([2, 0]),
                _ => {}
            }
        }
        // TC a352b1fa ObjectMgr.cpp:3325-3369: match ItemSpec stat pairs;
        // an item with no matching specialization is usable by everyone.
        let mut matched = false;
        for rule in &self.weapons {
            if level >= 110 && rule.max_level < 110 || level > 40 && rule.max_level <= 40 {
                continue;
            }
            if item.allowable_class != -1
                && item.allowable_class as u64 & (1_u64 << (rule.class - 1)) == 0
            {
                continue;
            }
            if (rule.primary == 40 || stats.contains(&rule.primary))
                && (rule.secondary == 40 || stats.contains(&rule.secondary))
            {
                matched = true;
                if rule.spec == spec {
                    return true;
                }
            }
        }
        !matched
    }
}

fn weapon_stats(subclass: u8) -> Vec<u32> {
    let stat = match subclass {
        0 => 7,
        1 => 8,
        2 => 16,
        3 => 15,
        4 => 11,
        5 => 12,
        6 => 19,
        7 => 9,
        8 => 10,
        9 => 28,
        10 => 18,
        13 => 14,
        15 => 13,
        16 => 20,
        18 => 17,
        19 => 21,
        _ => return Vec::new(),
    };
    vec![stat]
}
