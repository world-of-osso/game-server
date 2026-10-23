//! SIMC_TIER_ALIAS_ITEMSET_BINDING names; eligibility is the native set helper's.
use crate::equipment_effects::item_sets::read_set_spells;
use crate::item_templates::ItemTemplates;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn read_set_facts(
    world: &World,
    actor: Entity,
) -> Result<BTreeMap<String, bool>, String> {
    let active: BTreeSet<_> = read_set_spells(world, actor)
        .into_iter()
        .map(|(set, spell)| (set, spell.id))
        .collect();
    let catalog = &world.resource::<ItemTemplates>().item_sets;
    let mut facts = BTreeMap::new();
    for (tier, id) in parse_tier_aliases()? {
        let set = catalog
            .sets
            .get(&id)
            .ok_or_else(|| format!("pinned tier{tier} references absent native ItemSet{id}"))?;
        for spell in &set.spells {
            let key = format!("set_bonus.{tier}_{}pc", spell.threshold);
            let enabled = active.contains(&(id, spell.spell_id));
            *facts.entry(key).or_insert(false) |= enabled;
        }
    }
    Ok(facts)
}

fn parse_tier_aliases() -> Result<Vec<(&'static str, u32)>, String> {
    include_str!("set_aliases_76f8118.csv")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|row| {
            let (tier, id) = row
                .split_once(',')
                .ok_or("invalid pinned ItemSet alias row")?;
            let id = id
                .parse()
                .map_err(|error| format!("native set alias {row}: {error}"))?;
            Ok((tier, id))
        })
        .collect()
}
