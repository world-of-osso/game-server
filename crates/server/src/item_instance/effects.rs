//! Native equip/on-use spell identities; stat budgets cannot substitute for procs.
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemEffect {
    pub id: u32,
    pub spell_id: u32,
    pub trigger: u8,
    pub cooldown_ms: i32,
    pub category_cooldown_ms: i32,
    pub category: u32,
}

#[derive(Default)]
pub struct ItemEffectTables {
    pub effects: HashMap<u32, ItemEffect>,
    pub items: HashMap<u32, Vec<u32>>,
}

pub fn equipped_item_effects(
    item_id: u32,
    bonus_effect_ids: &[u32],
    tables: &ItemEffectTables,
) -> Result<Vec<ItemEffect>, String> {
    let base_ids = tables.items.get(&item_id).into_iter().flatten();
    base_ids
        .chain(bonus_effect_ids)
        .map(|id| {
            tables
                .effects
                .get(id)
                .copied()
                .ok_or_else(|| format!("equipped item {item_id}: native ItemEffect {id} absent"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_instance_missing_bonus_effect_is_an_error_not_silent_base_gear() {
        let error = equipped_item_effects(244774, &[234561], &ItemEffectTables::default());
        assert!(
            error.is_err(),
            "unresolved native ItemEffect must not disappear"
        );
    }
}
