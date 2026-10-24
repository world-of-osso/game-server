//! Shared native fixture machinery; source rows and spec facts belong to adapters.
#[cfg(test)]
mod composite_tests;
pub(crate) mod default_pet;
pub(crate) mod equipment;
pub(crate) mod execution;
#[cfg(test)]
mod gating_tests;
#[cfg(test)]
mod precombat_poll_tests;
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
    // Literal control must not keep the retired kit's owner-follow movement.
    world
        .entity_mut(actor)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(LiteralParityFixture);
    Ok(())
}

pub(crate) enum PrecombatPoll {
    Waiting,
    Selected {
        action: Action,
        execution: Execution,
    },
    Finished,
}

pub(crate) struct Driver {
    program: Program,
    variables: Variables,
    precombat_cursor: usize,
    selected_precombat: bool,
    precombat_harmful_executed: bool,
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
            precombat_cursor: 0,
            selected_precombat: false,
            precombat_harmful_executed: false,
            snapshot: None,
            cancellation: None,
            decisions: 0,
        }
    }

    /// The caller advances native frames and resamples without consuming busy rows.
    pub fn poll_precombat(
        &mut self,
        facts: &dyn ActionFacts,
        prepared: &Prepared,
        equipment: &Equipment,
    ) -> Result<PrecombatPoll, String> {
        if prepared.setup_pending() {
            return Ok(PrecombatPoll::Waiting);
        }
        Ok(
            match self.select("precombat", facts, prepared, equipment)? {
                Some((action, execution)) => PrecombatPoll::Selected { action, execution },
                None => PrecombatPoll::Finished,
            },
        )
    }

    /// Evaluate only after native readiness has been sampled with mutable World.
    pub fn select(
        &mut self,
        list: &str,
        facts: &dyn ActionFacts,
        prepared: &Prepared,
        equipment: &Equipment,
    ) -> Result<Option<(Action, Execution)>, String> {
        self.selected_precombat = list == "precombat";
        if self.selected_precombat && prepared.setup_pending() {
            return Err("PRECOMBAT_WAIT_REQUIRED: advance native frames via poll_precombat".into());
        }
        if self.selected_precombat && self.precombat_harmful_executed {
            return Ok(None);
        }
        let mut usable = |action: &Action| {
            Ok(prepared.permits_row(action)?
                && facts.target_ready(action)?
                && prepared.execution(action, equipment)?.is_some())
        };
        let action = if list == "precombat" {
            self.program.select_next(
                list,
                &mut self.precombat_cursor,
                facts,
                &mut self.variables,
                &mut usable,
            )?
        } else {
            self.program
                .select(list, facts, &mut self.variables, &mut usable)?
        };
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
        let ends_precombat = self.selected_precombat && harmful_execution(world, &execution)?;
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
        self.precombat_harmful_executed |= ends_precombat;
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

fn harmful_execution(world: &World, execution: &Execution) -> Result<bool, String> {
    let id = match execution {
        Execution::Composite(_) => {
            return Err("composite precombat requires a clock-aware executor".into());
        }
        Execution::Snapshot => return Ok(false),
        Execution::AutoAttack(_) | Execution::RetargetAutoAttack(_) => return Ok(true),
        Execution::Spell(id, _) => *id,
        Execution::Item(item, _) => item.on_use.spell_id,
    };
    let store = world.resource::<crate::spell_info::SpellStore>();
    let spell = store
        .get(id)
        .ok_or_else(|| format!("native precombat spell{id} missing"))?;
    Ok(crate::spell_info::positivity::negative_effects(store, spell) != 0)
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
