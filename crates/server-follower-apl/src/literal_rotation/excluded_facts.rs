use bevy::prelude::Entity;
use server::simc_apl::{Action, ActionFacts, Facts};
use std::collections::BTreeSet;

pub(super) struct ExcludedFacts<'a> {
    pub inner: &'a dyn ActionFacts,
    pub excluded: &'a BTreeSet<Entity>,
    pub default_target: Entity,
}

impl ExcludedFacts<'_> {
    fn bind(&self, action: &Action) -> Action {
        let mut bound = action.clone();
        if bound.target.is_none() {
            bound.target = Some(self.default_target.to_bits());
        }
        bound
    }
}

impl Facts for ExcludedFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.inner.value(name)
    }
}

impl ActionFacts for ExcludedFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        self.inner.value_for(&self.bind(action), name)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.inner.target_ready(&self.bind(action))
    }
    fn primary_resource(&self) -> Result<String, String> {
        self.inner.primary_resource()
    }
    fn ready_at_resource_max(&self, action: &Action, resource: &str) -> Result<bool, String> {
        self.inner
            .ready_at_resource_max(&self.bind(action), resource)
    }
    fn action_variants(&self, action: &Action) -> Result<Vec<Action>, String> {
        self.inner.action_variants(&self.bind(action))
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        Ok(self
            .inner
            .target_ids(action)?
            .into_iter()
            .filter(|bits| {
                Entity::try_from_bits(*bits).is_some_and(|target| !self.excluded.contains(&target))
            })
            .collect())
    }
}
