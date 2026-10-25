//! TrinityCore a352b1fa SmartScript::SetTimedActionList / UpdateTimer.
use super::*;
use crate::random as rand;

fn roll_initial_delay(event: &SmartEvent) -> u32 {
    let [min, max, ..] = event.event_params;
    if max > min {
        rand::thread_rng().gen_range(min..=max)
    } else {
        min
    }
}

fn roll_action_chance(event: &SmartEvent) -> bool {
    event.chance == 0 || event.chance >= 100 || rand::thread_rng().gen_range(0..100) < event.chance
}

impl SmartAi {
    pub(super) fn tick_hook_cooldown(&mut self, index: usize, diff_ms: u32) -> bool {
        let event = &self.events[index];
        let hook = matches!(
            event.event_type,
            EVENT_SUMMONED_UNIT | EVENT_SUMMON_DESPAWNED | EVENT_COUNTER_SET
        );
        if !hook || !self.in_phase(event.phase_mask) {
            return false;
        }
        let state = &mut self.states[index];
        if state.timer_ms >= diff_ms {
            state.timer_ms -= diff_ms;
        } else {
            state.active = true;
        }
        true
    }

    pub(super) fn start_timed_list(
        &mut self,
        ctx: &Ctx,
        event: &SmartEvent,
        invoker: Option<Entity>,
    ) {
        if self.processing_timed_list {
            warn!("SmartAI cannot overwrite its own timed list from a timed action");
            return;
        }
        let [id, timer_type, allow_override, ..] = event.action_params;
        if self.timed_list.is_some() && allow_override == 0 {
            return;
        }
        let Some(events) = ctx.world.resource::<SmartScripts>().1.get(&id).cloned() else {
            warn!(id, "SmartAI timed action list is absent");
            return;
        };
        let timer_ms = events.first().map_or(0, roll_initial_delay);
        self.timed_list = Some(TimedList {
            events,
            index: 0,
            timer_ms,
            invoker,
            timer_type,
        });
    }

    pub(super) fn update_timed_list(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        let Some(mut list) = self.timed_list.take() else {
            return;
        };
        let combat_gate = match list.timer_type {
            0 => !ctx.is_engaged(),
            1 => ctx.is_engaged(),
            _ => true,
        };
        if !combat_gate
            || !list
                .events
                .get(list.index)
                .is_some_and(|event| self.in_phase(event.phase_mask))
        {
            self.timed_list = Some(list);
            return;
        }
        if list.timer_ms > diff_ms {
            list.timer_ms -= diff_ms;
            self.timed_list = Some(list);
            return;
        }
        self.processing_timed_list = true;
        let pending = self.execute_due_timed_actions(ctx, &mut list);
        self.processing_timed_list = false;
        if pending {
            self.timed_list = Some(list);
        }
    }

    fn execute_due_timed_actions(&mut self, ctx: &mut Ctx, list: &mut TimedList) -> bool {
        while let Some(event) = list.events.get(list.index) {
            if event.action_type == ACTION_CAST && ctx.is_casting() {
                list.timer_ms = 1;
                return true;
            }
            if roll_action_chance(event) && !self.execute_action(ctx, event, list.invoker) {
                list.timer_ms = 1;
                return true;
            }
            list.index += 1;
            let Some(next) = list.events.get(list.index) else {
                return false;
            };
            list.timer_ms = roll_initial_delay(next);
            if list.timer_ms > 0 {
                return true;
            }
        }
        false
    }
}
