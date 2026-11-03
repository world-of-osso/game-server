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
    fn missing_native_target_health_is_recorded_and_selection_continues() {
        use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
        let mut world = World::new();
        let actor = world.spawn_empty().id();
        let target = world.spawn_empty().id();
        let targets = [target];
        let bindings = Bindings::default();
        let native = NativeFacts::new(&world, actor, &bindings, Encounter::dungeon(0.0), 1)
            .with_targets(Some(target), &targets);
        let errors = FactErrors::default();
        let facts = FactContext {
            inner: &native,
            errors: &errors,
            actor,
            default_target: Some(target),
        };
        let program = Program::parse(&[
            ("default", "interrupt,if=target.health>0"),
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
        let entries = errors.snapshot();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].fact, "target.health");
        assert_eq!(entries[0].target, Some(target.to_bits()));
        assert!(entries[0].detail.contains("lacks Health"));
    }

    #[test]
    fn cycling_variable_does_not_keep_partial_updates_after_a_fact_error() {
        struct Targets([u64; 2]);
        impl Facts for Targets {
            fn value(&self, name: &str) -> Result<f64, String> {
                match name {
                    "active_enemies" => Ok(2.0),
                    _ => Err(format!("unexpected fact {name}")),
                }
            }
        }
        impl ActionFacts for Targets {
            fn target_ids(&self, _: &Action) -> Result<Vec<u64>, String> {
                Ok(self.0.to_vec())
            }
            fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
                if name == "target.health" {
                    if action.target == Some(self.0[0]) {
                        Ok(20.0)
                    } else {
                        Err("target has no Health".into())
                    }
                } else {
                    self.value(name)
                }
            }
        }
        let mut world = World::new();
        let actor = world.spawn_empty().id();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let targets = Targets([first.to_bits(), second.to_bits()]);
        let errors = FactErrors::default();
        let facts = FactContext {
            inner: &targets,
            errors: &errors,
            actor,
            default_target: Some(first),
        };
        let program = Program::parse(&[
            ("default", "variable,name=x,op=set,value=5"),
            (
                "default",
                "cycling_variable,name=x,op=max,value=target.health",
            ),
            ("default", "wrong,if=variable.x>10"),
            ("default", "fallback"),
        ])
        .unwrap();
        let mut state = program.state();
        let selected = program
            .select("default", &facts, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(selected.name, "fallback");
        assert_eq!(state.value("x"), Some(5.0));
        let entries = errors.snapshot();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].target, Some(second.to_bits()));
        assert_eq!(entries[0].count, 1);
    }

    #[test]
    fn native_cast_reaction_delay_does_not_enter_the_error_ledger() {
        use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
        use shared::casting::CastState;
        let mut h = server::spell_cast::test_harness::harness(Vec::new(), &[133], 1).unwrap();
        let actor = h.caster;
        let target = h.npc;
        h.app
            .world_mut()
            .entity_mut(target)
            .insert(CastState::normal(133, actor.to_bits(), 3.0, true));
        h.app.world_mut().flush();
        let errors = FactErrors::default();
        let program = Program::parse(&[
            ("default", "interrupt,if=target.debuff.casting.react"),
            ("default", "attack"),
        ])
        .unwrap();
        let bindings = Bindings::default();
        let targets = [target];
        let select = |world: &World| {
            let native = NativeFacts::new(world, actor, &bindings, Encounter::dungeon(0.0), 1)
                .with_targets(Some(target), &targets);
            let facts = FactContext {
                inner: &native,
                errors: &errors,
                actor,
                default_target: Some(target),
            };
            program
                .select("default", &facts, &mut program.state(), |_| Ok(true))
                .unwrap()
                .unwrap()
                .name
        };
        assert_eq!(select(h.app.world()), "attack");
        assert!(errors.snapshot().is_empty());
        h.app
            .world_mut()
            .get_mut::<CastState>(target)
            .unwrap()
            .elapsed = 2.0;
        assert_eq!(select(h.app.world()), "interrupt");
        assert!(errors.snapshot().is_empty());
    }

    #[test]
    fn pending_native_navigation_does_not_enter_the_error_ledger() {
        use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
        use server::navigation::{Goal, MoveRequest, Navigation, Step};
        use shared::components::{CreatureMotion, Position};

        let mut world = World::new();
        let from = Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let to = Position {
            x: 5.0,
            y: 0.0,
            z: 0.0,
        };
        let actor = world.spawn((from, CreatureMotion::Run)).id();
        let target = world.spawn(to).id();
        let mut navigation =
            Navigation::new(std::path::Path::new("/nonexistent-navigation-fixture"));
        assert!(matches!(
            navigation.advance(
                actor,
                MoveRequest {
                    map_id: 34,
                    goal: Goal::Chase(target.to_bits()),
                    from,
                    to,
                    distance: 1.0,
                    reach: 2.0,
                }
            ),
            Step::Pending
        ));
        world.insert_resource(navigation);
        let bindings = Bindings::default();
        let native = NativeFacts::new(&world, actor, &bindings, Encounter::dungeon(0.0), 1);
        let errors = FactErrors::default();
        let facts = FactContext {
            inner: &native,
            errors: &errors,
            actor,
            default_target: Some(target),
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
