//! Per-equipped-item source condition context; shared by every literal adapter.
use super::equipment::Equipment;
use server::simc_apl::{Action, ActionFacts, Facts, Program};

pub(super) struct ItemContext<'a> {
    pub facts: &'a dyn ActionFacts,
    pub equipment: &'a Equipment,
    pub program: &'a Program,
}

impl Facts for ItemContext<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.facts.value(name)
    }
}

impl ActionFacts for ItemContext<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        // SimC76f8118 player.cpp10169–10194 binds both aliases to this use_item.
        let relative = name
            .strip_prefix("this_trinket.")
            .map(|field| (field, false))
            .or_else(|| {
                name.strip_prefix("other_trinket.")
                    .map(|field| (field, true))
            });
        let Some((field, other)) = relative else {
            return self.facts.value_for(action, name);
        };
        let slot = self
            .equipment
            .slot_for_action(action)?
            .ok_or("trinket expression lacks equipped item context")?;
        let index = match (slot, other) {
            ("trinket1", false) | ("trinket2", true) => 1,
            ("trinket2", false) | ("trinket1", true) => 2,
            _ => {
                return Err(format!(
                    "trinket expression unsupported for item slot {slot}"
                ));
            }
        };
        self.facts
            .value_for(action, &format!("trinket.{index}.{field}"))
    }

    fn action_variants(&self, action: &Action) -> Result<Vec<Action>, String> {
        if action.name != "use_items" {
            return self.facts.action_variants(action);
        }
        Ok(self
            .equipment
            .use_items_slots(action, self.program)?
            .into_iter()
            .map(|slot| {
                let mut proxy = action.clone();
                proxy.options.insert("slot".into(), slot.into());
                proxy
            })
            .collect())
    }

    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.facts.target_ready(action)
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.facts.target_ids(action)
    }
    fn primary_resource(&self) -> Result<String, String> {
        self.facts.primary_resource()
    }
    fn ready_at_resource_max(&self, action: &Action, resource: &str) -> Result<bool, String> {
        self.facts.ready_at_resource_max(action, resource)
    }
}
