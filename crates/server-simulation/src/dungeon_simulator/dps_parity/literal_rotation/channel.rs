//! SimC76f8118 dot.cpp707–735/815–869/980–994 channel decisions.
use super::*;
use shared::casting::{CastState, CastType};

pub(crate) enum Decision {
    Cancel,
    Chain(Action, Execution),
}

pub(super) struct Channel {
    action: Action,
    spell: u32,
    root: String,
    interrupt: bool,
    interrupt_if: Option<Expression>,
    early_chain_if: Option<Expression>,
    immediate: bool,
    global: bool,
    chain: bool,
    last_tick: u32,
}

fn flag(action: &Action, key: &str) -> Result<bool, String> {
    match action.options.get(key).map(String::as_str) {
        None | Some("0") => Ok(false),
        Some("1") => Ok(true),
        Some(value) => Err(format!("unsupported channel option {key}={value}")),
    }
}

impl Channel {
    pub(super) fn read(action: &Action, spell: u32, root: &str) -> Result<Option<Self>, String> {
        let interrupt_if = action
            .options
            .get("interrupt_if")
            .map(|text| Expression::parse(text))
            .transpose()?;
        let early_chain_if = action
            .options
            .get("early_chain_if")
            .map(|text| Expression::parse(text))
            .transpose()?;
        let interrupt = flag(action, "interrupt")?;
        let chain = flag(action, "chain")?;
        let watches_channel =
            interrupt || interrupt_if.is_some() || chain || early_chain_if.is_some();
        if !watches_channel {
            return Ok(None);
        }
        Ok(Some(Self {
            action: action.clone(),
            spell,
            root: root.into(),
            interrupt,
            interrupt_if,
            early_chain_if,
            immediate: flag(action, "interrupt_immediate")?,
            global: flag(action, "interrupt_global")?,
            chain,
            last_tick: 0,
        }))
    }

    fn observe_tick(&mut self, cast: &CastState) -> Result<bool, String> {
        if cast.channel_tick_interval <= 0.0 {
            return Err("native channel has no tick period".into());
        }
        let tick =
            ((cast.elapsed - cast.channel_tick_timer) / cast.channel_tick_interval).round() as u32;
        if tick <= self.last_tick {
            return Ok(false);
        }
        self.last_tick = tick;
        Ok(true)
    }

    fn requests_interruption(
        &self,
        facts: &dyn ActionFacts,
        variables: &Variables,
    ) -> Result<bool, String> {
        if self.interrupt {
            return Ok(true);
        }
        let expression = self
            .interrupt_if
            .as_ref()
            .map(|expression| {
                expression.evaluate(&BoundAction {
                    facts,
                    action: &self.action,
                    variables,
                })
            })
            .transpose()?
            .is_some_and(|value| value != 0.0);
        Ok(expression)
    }

    fn interrupt_ready(&self, facts: &dyn ActionFacts) -> Result<bool, String> {
        let gcd_ready = facts.value_for(&self.action, "gcd.remains")? <= 0.0;
        Ok(gcd_ready || self.immediate)
    }

    fn requests_chain(
        &self,
        cast: &CastState,
        facts: &dyn ActionFacts,
        variables: &Variables,
    ) -> Result<bool, String> {
        // Native cast timers are rounded to milliseconds, like SimC timespan.
        let remaining_ms = ((cast.duration - cast.elapsed) * 1000.0).round() as i64;
        let interval_ms = (cast.channel_tick_interval * 1000.0).round() as i64;
        if self.chain && remaining_ms <= interval_ms {
            return Ok(true);
        }
        // SimC76f8118 dot.cpp841–845: early_chain_if is independent of chain=1,
        // but requires a paid tick (observe_tick) and the actual ready GCD.
        let Some(expression) = &self.early_chain_if else {
            return Ok(false);
        };
        let early = expression.evaluate(&BoundAction {
            facts,
            action: &self.action,
            variables,
        })? != 0.0;
        Ok(early && facts.value_for(&self.action, "gcd.remains")? <= 0.0)
    }
}

impl Driver {
    pub fn execute_channel_decision(
        &mut self,
        world: &mut World,
        actor: Entity,
        decision: Decision,
    ) -> Result<(), String> {
        crate::spell_cast::cancel_cast(world, actor);
        match decision {
            Decision::Cancel => Ok(()),
            Decision::Chain(action, execution) => self.execute(world, actor, action, execution),
        }
    }

    /// Sample after native ticks with Prepared::read_after_channel. The query
    /// never stops a channel, advances a cooldown, grants power, or casts.
    pub fn channel_decision(
        &mut self,
        world: &World,
        actor: Entity,
        facts: &dyn ActionFacts,
        prepared: &Prepared,
        equipment: &Equipment,
    ) -> Result<Option<Decision>, String> {
        let Some(mut channel) = self.channel.take() else {
            return Ok(None);
        };
        let Some(cast) = world
            .get::<CastState>(actor)
            .filter(|cast| cast.spell_id == channel.spell && cast.cast_type == CastType::Channel)
        else {
            return Ok(None);
        };
        let decision = self.select_channel_decision(&mut channel, cast, facts, prepared, equipment);
        self.channel = Some(channel);
        decision
    }

    fn select_channel_decision(
        &mut self,
        channel: &mut Channel,
        cast: &CastState,
        facts: &dyn ActionFacts,
        prepared: &Prepared,
        equipment: &Equipment,
    ) -> Result<Option<Decision>, String> {
        if !channel.observe_tick(cast)? {
            return Ok(None);
        }
        // dot.cpp729–734: first evaluate the interrupt request and GCD guard;
        // neither grants permission without a genuinely ready higher action.
        if channel.requests_interruption(facts, &self.variables)? {
            let ready = channel.interrupt_ready(facts)?;
            if self.has_higher_priority(channel, facts, prepared, equipment)? && ready {
                return Ok(Some(Decision::Cancel));
            }
        }
        if !channel.requests_chain(cast, facts, &self.variables)? {
            return Ok(None);
        }
        let decision = self.prepare_chain(channel, facts, prepared, equipment)?;
        if decision.is_none() {
            return Ok(None);
        }
        // dot.cpp845: current-row native readiness precedes the priority scan.
        if self.has_higher_priority(channel, facts, prepared, equipment)? {
            self.pending_line = None;
            return Ok(None);
        }
        Ok(decision)
    }

    fn has_higher_priority(
        &mut self,
        channel: &Channel,
        facts: &dyn ActionFacts,
        prepared: &Prepared,
        equipment: &Equipment,
    ) -> Result<bool, String> {
        let item_context = item_context::ItemContext {
            facts,
            equipment,
            program: &self.program,
        };
        let facts: &dyn ActionFacts = &item_context;
        let mut usable = |action: &Action| {
            candidate_ready(
                action,
                facts,
                prepared,
                equipment,
                &self.program,
                &self.line_ready_at,
            )
        };
        self.program
            .select_before(
                &channel.root,
                &channel.action,
                channel.global,
                facts,
                &mut self.variables,
                &mut usable,
            )
            .map(|action| action.is_some())
    }

    fn prepare_chain(
        &mut self,
        channel: &Channel,
        facts: &dyn ActionFacts,
        prepared: &Prepared,
        equipment: &Equipment,
    ) -> Result<Option<Decision>, String> {
        let row = channel
            .action
            .source_row
            .ok_or("chained action lacks source row")?;
        let item_context = item_context::ItemContext {
            facts,
            equipment,
            program: &self.program,
        };
        let facts: &dyn ActionFacts = &item_context;
        let mut usable = |action: &Action| {
            candidate_ready(
                action,
                facts,
                prepared,
                equipment,
                &self.program,
                &self.line_ready_at,
            )
        };
        let Some(action) = self
            .program
            .select_row(row, facts, &mut self.variables, &mut usable)?
        else {
            return Ok(None);
        };
        let execution = prepared
            .execution_in(&action, equipment, &self.program)?
            .ok_or("ready chained action lost native execution")?;
        self.pending_line = line_deadline(&action, facts, &self.variables)?;
        self.selected_list = channel.root.clone();
        Ok(Some(Decision::Chain(action, execution)))
    }
}
