//! Shared native fixture machinery; source rows and spec facts belong to adapters.
#[cfg(test)]
mod composite_tests;
pub(crate) mod equipment;
pub(crate) mod execution;
#[cfg(test)]
mod tests;
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;
use equipment::Equipment;
use execution::{Execution, Prepared};
use serde_json::Value;
use server::simc_apl::{Action, ActionFacts, Expression, Facts, Program, Variables};

pub(crate) use server::test_support::LiteralParityFixture;

pub(crate) fn capture_fixture(
    world: &mut World,
    actor: Entity,
    encounter: Encounter,
) -> Result<(), String> {
    if !matches!(encounter, Encounter::Fixture { .. }) {
        return Err(
            "DUNGEON_TTD_UNAVAILABLE: literal controller requires an explicit Fixture encounter"
                .into(),
        );
    }
    world.entity_mut(actor).insert(LiteralParityFixture);
    Ok(())
}

pub(crate) struct Driver {
    program: Program,
    variables: Variables,
    pub snapshot: Option<Value>,
    cancellation: Option<(Action, Expression, u32)>,
    pub decisions: u64,
}

impl Driver {
    pub fn new(program: Program) -> Self {
        let variables = program.state();
        Self {
            program,
            variables,
            snapshot: None,
            cancellation: None,
            decisions: 0,
        }
    }

    /// Evaluate only after native readiness has been sampled with mutable World.
    pub fn select(
        &mut self,
        list: &str,
        facts: &dyn ActionFacts,
        prepared: &Prepared,
        equipment: &Equipment,
    ) -> Result<Option<(Action, Execution)>, String> {
        let action = self
            .program
            .select(list, facts, &mut self.variables, |action| {
                Ok(prepared.execution(action, equipment)?.is_some())
            })?;
        let Some(action) = action else {
            return Ok(None);
        };
        let execution = prepared
            .execution(&action, equipment)?
            .ok_or("selected literal action lost native readiness")?;
        Ok(Some((action, execution)))
    }

    pub fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: Execution,
    ) -> Result<(), String> {
        let cancellation = if let Execution::Spell(id, _) = &execution {
            action
                .options
                .get("cancel_if")
                .map(|expression| {
                    Expression::parse(expression).map(|expr| (action.clone(), expr, *id))
                })
                .transpose()?
        } else {
            None
        };
        if matches!(execution, Execution::Snapshot) {
            self.snapshot = Some(super::report::stats(world, actor));
        } else {
            execution::execute(world, actor, execution)?;
        }
        if cancellation.is_some() {
            self.cancellation = cancellation;
        }
        self.decisions += 1;
        Ok(())
    }

    /// Caller releases its immutable facts borrow before cancelling the actual cast.
    pub fn should_cancel(
        &mut self,
        world: &World,
        actor: Entity,
        facts: &dyn ActionFacts,
    ) -> Result<bool, String> {
        let Some((action, expression, spell)) = &self.cancellation else {
            return Ok(false);
        };
        if !world
            .get::<shared::casting::CastState>(actor)
            .is_some_and(|cast| cast.spell_id == *spell)
        {
            self.cancellation = None;
            return Ok(false);
        }
        let cancel = expression.evaluate(&BoundAction { facts, action })? != 0.0;
        if cancel {
            self.cancellation = None;
        }
        Ok(cancel)
    }
}

struct BoundAction<'a> {
    facts: &'a dyn ActionFacts,
    action: &'a Action,
}
impl Facts for BoundAction<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.facts.value_for(self.action, name)
    }
}
