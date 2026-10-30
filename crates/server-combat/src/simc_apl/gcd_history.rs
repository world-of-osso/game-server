//! Bounded successful-GCD history configured from a loaded APL's lookback.
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Component, Debug, Clone, Default)]
pub struct GcdHistory {
    limit: usize,
    casts: VecDeque<u32>,
}

impl GcdHistory {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            casts: VecDeque::new(),
        }
    }

    pub fn record(&mut self, spell_id: u32) {
        if self.limit == 0 {
            return;
        }
        if self.casts.len() == self.limit {
            self.casts.pop_front();
        }
        self.casts.push_back(spell_id);
    }

    pub fn previous(&self, index: usize) -> Result<Option<u32>, String> {
        if index == 0 || index > self.limit {
            return Err(format!(
                "APL GCD lookback {index} exceeds configured history {}",
                self.limit
            ));
        }
        Ok(self
            .casts
            .len()
            .checked_sub(index)
            .and_then(|offset| self.casts.get(offset).copied()))
    }
}

/// Install the loaded program's exact lookback before its first cast.
pub fn initialize(world: &mut World, actor: Entity, program: &crate::simc_apl::Program) {
    world
        .entity_mut(actor)
        .insert(GcdHistory::new(program.gcd_history_limit()));
}

/// Preserve observed casts when a controller resumes with the same program.
pub fn ensure_initialized(
    world: &mut World,
    actor: Entity,
    program: &crate::simc_apl::Program,
) -> Result<(), String> {
    if let Some(history) = world.get::<GcdHistory>(actor) {
        if history.limit < program.gcd_history_limit() {
            return Err("APL history was initialized with a shorter lookback".into());
        }
    } else {
        initialize(world, actor, program);
    }
    Ok(())
}

/// Find the greatest prev_gcd index in every literal row, including action
/// options such as channel interruption expressions.
pub fn required_lookback<'a>(rows: impl IntoIterator<Item = &'a str>) -> Result<usize, String> {
    let mut maximum = 0;
    for row in rows {
        let lower = row.to_ascii_lowercase();
        for suffix in lower.split("prev_gcd.").skip(1) {
            let digits: String = suffix.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() || !suffix[digits.len()..].starts_with('.') {
                return Err(format!("invalid prev_gcd operand in {row:?}"));
            }
            let index: usize = digits
                .parse()
                .map_err(|error| format!("invalid prev_gcd index {digits}: {error}"))?;
            if index == 0 {
                return Err("prev_gcd index must be positive".into());
            }
            maximum = maximum.max(index);
        }
    }
    Ok(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_follower_program_lookback_bounds_successful_cast_history() {
        let program = crate::simc_apl::Program::parse(&[
            ("default", "strike,if=prev_gcd.1.judgment"),
            ("default", "channel,interrupt_if=prev_gcd.3.strike"),
        ])
        .unwrap();
        let limit = program.gcd_history_limit();
        assert_eq!(limit, 3);
        let mut world = World::new();
        let actor = world.spawn_empty().id();
        initialize(&mut world, actor, &program);
        let mut history = world.get_mut::<GcdHistory>(actor).unwrap();
        for spell in [1, 2, 3, 4] {
            history.record(spell);
        }
        assert_eq!(history.previous(1).unwrap(), Some(4));
        assert_eq!(history.previous(3).unwrap(), Some(2));
        assert!(history.previous(4).is_err());
        assert!(required_lookback(["strike,if=prev_gcd.0.strike"]).is_err());
    }
}
