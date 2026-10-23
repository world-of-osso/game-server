//! Literal pinned Windwalker fixture driver; Stockade retains its native kit.
mod bindings;
mod equipment;
mod execution;
mod facts;
mod source;
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use equipment::Equipment;
use execution::{Execution, Prepared};
use facts::WwFacts;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts, Expression, Facts, Program, Variables};

pub(super) struct Rotation {
    program: Program,
    variables: Variables,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    snapshot: Option<Value>,
    cancellation: Option<(Action, Expression, u32)>,
    pub decisions: u64,
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        if !matches!(encounter, Encounter::Fixture { .. }) {
            return Err(
                "DUNGEON_TTD_UNAVAILABLE: literal WW requires an explicit Fixture encounter".into(),
            );
        }
        if !world
            .get::<crate::followers::Follower>(actor)
            .is_some_and(|f| f.profile.spec_id == 269)
        {
            return Err("literal WW fixture requires its native spec269 follower".into());
        }
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let program = source::load()?;
        let variables = program.state();
        let mut rotation = Self {
            program,
            variables,
            bindings,
            equipment,
            encounter,
            snapshot: None,
            cancellation: None,
            decisions: 0,
        };
        rotation.precombat(world, actor)?;
        // Only this explicit dps_parity Fixture setup owns controller capture.
        // Rejected dungeon loads above cannot attach it or mutate follower identity.
        world
            .entity_mut(actor)
            .insert(crate::followers::simulator::LiteralParityFixture);
        Ok(rotation)
    }

    fn precombat(&mut self, world: &mut World, actor: Entity) -> Result<(), String> {
        loop {
            let prepared =
                Prepared::read(world, actor, &[], &self.equipment, self.snapshot.is_none())?;
            let Some((action, execution)) =
                self.select(world, actor, &[], "precombat", &prepared)?
            else {
                return Ok(());
            };
            self.execute(world, actor, action, execution)?;
        }
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if targets.is_empty() {
            return Err("literal WW fixture has no native target".into());
        }
        self.cancel_channel(world, actor, targets)?;
        let prepared = Prepared::read(world, actor, targets, &self.equipment, false)?;
        if let Some((action, execution)) =
            self.select(world, actor, targets, "default", &prepared)?
        {
            self.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    fn select(
        &mut self,
        world: &World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        prepared: &Prepared,
    ) -> Result<Option<(Action, Execution)>, String> {
        let facts = WwFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets.len(),
            prepared.xuen_active,
        );
        let action = self
            .program
            .select(list, &facts, &mut self.variables, |action| {
                Ok(prepared.execution(action, &self.equipment)?.is_some())
            })?;
        let Some(action) = action else {
            return Ok(None);
        };
        let execution = prepared
            .execution(&action, &self.equipment)?
            .ok_or("selected literal WW action lost native readiness")?;
        Ok(Some((action, execution)))
    }

    fn execute(
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

    fn cancel_channel(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let Some((action, expression, spell)) = &self.cancellation else {
            return Ok(());
        };
        if !world
            .get::<shared::casting::CastState>(actor)
            .is_some_and(|cast| cast.spell_id == *spell)
        {
            self.cancellation = None;
            return Ok(());
        }
        let xuen = execution::read_xuen(world, actor);
        let facts = WwFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets.len(),
            xuen,
        );
        if expression.evaluate(&BoundAction {
            facts: &facts,
            action,
        })? != 0.0
        {
            crate::spell_cast::cancel_cast(world, actor);
            self.cancellation = None;
        }
        Ok(())
    }

    pub fn value_for(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        action: &Action,
        name: &str,
    ) -> Result<f64, String> {
        let xuen = execution::read_xuen(world, actor);
        WwFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets.len(),
            xuen,
        )
        .value_for(action, name)
    }

    pub fn report(&self) -> Value {
        json!({"driver": "LITERAL_SIMC_WINDWALKER_76f8118", "decisions": self.decisions,
            "native_precombat_snapshot": self.snapshot,
            "named_policy": "SIMC_MISSING_COOLDOWN_PLACEHOLDER",
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion",
                "NATIVE_WW_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_ITEM_PROC_QUALIFICATION_OPEN"]})
    }
}

struct BoundAction<'a> {
    facts: &'a WwFacts<'a>,
    action: &'a Action,
}
impl Facts for BoundAction<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.facts.value_for(self.action, name)
    }
}

#[cfg(test)]
mod tests;
