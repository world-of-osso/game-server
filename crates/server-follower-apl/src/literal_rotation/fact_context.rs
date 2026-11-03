use bevy::prelude::Entity;
use server::simc_apl::{Action, ActionFacts, FactErrors, Facts};

/// Record unavailable facts, then let selection continue with the next row.
pub(super) struct FactContext<'a> {
    pub inner: &'a dyn ActionFacts,
    pub errors: &'a FactErrors,
    pub actor: Entity,
    pub default_target: Option<Entity>,
}

impl FactContext<'_> {
    fn record<T>(
        &self,
        fact: &str,
        target: Option<Entity>,
        result: Result<T, String>,
    ) -> Result<T, String> {
        result.map_err(|error| {
            if server::simc_apl::is_not_ready(&error) {
                return error;
            }
            self.errors.record(
                fact,
                self.actor.to_bits(),
                target.map(|target| target.to_bits()),
                &error,
            );
            server::simc_apl::not_ready("recorded APL fact failure")
        })
    }

    fn target(&self, action: &Action) -> Option<Entity> {
        action
            .target
            .and_then(Entity::try_from_bits)
            .or(self.default_target)
    }
}

impl Facts for FactContext<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.record(name, self.default_target, self.inner.value(name))
    }
}

impl ActionFacts for FactContext<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        self.record(
            name,
            self.target(action),
            self.inner.value_for(action, name),
        )
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.record(
            "target_ready",
            self.target(action),
            self.inner.target_ready(action),
        )
    }
    fn primary_resource(&self) -> Result<String, String> {
        self.record(
            "primary_resource",
            self.default_target,
            self.inner.primary_resource(),
        )
    }
    fn ready_at_resource_max(&self, action: &Action, resource: &str) -> Result<bool, String> {
        self.record(
            "resource_max",
            self.target(action),
            self.inner.ready_at_resource_max(action, resource),
        )
    }
    fn action_variants(&self, action: &Action) -> Result<Vec<Action>, String> {
        self.inner.action_variants(action)
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.record(
            "target_ids",
            self.target(action),
            self.inner.target_ids(action),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::World;
    use server::simc_apl::Program;

    struct Missing {
        pending: bool,
    }
    impl Facts for Missing {
        fn value(&self, _: &str) -> Result<f64, String> {
            Err(if self.pending {
                server::simc_apl::not_ready("DUNGEON_MOVEMENT_DESTINATION_PENDING")
            } else {
                "target has no Health".into()
            })
        }
    }
    impl ActionFacts for Missing {
        fn value_for(&self, _: &Action, name: &str) -> Result<f64, String> {
            self.value(name)
        }
    }

    #[test]
    fn missing_fact_is_recorded_and_lower_priority_action_is_selected() {
        let mut world = World::new();
        let actor = world.spawn_empty().id();
        let target = world.spawn_empty().id();
        let errors = FactErrors::default();
        let missing = Missing { pending: false };
        let facts = FactContext {
            inner: &missing,
            errors: &errors,
            actor,
            default_target: Some(target),
        };
        let program = Program::parse(&[
            ("default", "interrupt,if=target.health>0"),
            ("default", "attack"),
        ])
        .unwrap();
        for _ in 0..2 {
            assert_eq!(
                program
                    .select("default", &facts, &mut program.state(), |_| Ok(true))
                    .unwrap()
                    .unwrap()
                    .name,
                "attack"
            );
        }
        let entries = errors.snapshot();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            (
                &*entries[0].fact,
                entries[0].actor,
                entries[0].target,
                entries[0].count
            ),
            ("target.health", actor.to_bits(), Some(target.to_bits()), 2)
        );
    }

    #[test]
    fn pending_movement_does_not_enter_the_error_ledger() {
        let mut world = World::new();
        let actor = world.spawn_empty().id();
        let errors = FactErrors::default();
        let missing = Missing { pending: true };
        let facts = FactContext {
            inner: &missing,
            errors: &errors,
            actor,
            default_target: None,
        };
        let program = Program::parse(&[
            ("default", "interrupt,if=movement.distance>0"),
            ("default", "attack"),
        ])
        .unwrap();
        assert_eq!(
            program
                .select("default", &facts, &mut program.state(), |_| Ok(true))
                .unwrap()
                .unwrap()
                .name,
            "attack"
        );
        assert!(errors.snapshot().is_empty());
    }
}
