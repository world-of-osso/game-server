//! SimC76f8118 sc_shaman.cpp556–629/11764/11830–11845/13057–13098:
//! actual MW consumption draws independent spaced Tempest and Storm Unleashed decks.
use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::auras::{self, ActiveAuras};
use crate::class_progression::KnownSpells;
use crate::random::Rng;
use crate::spell_cast::{self, effects::EffectCtx};

const MAX_DRAW: usize = 10;

struct SpendingProc {
    talent: u32,
    buff: u32,
    cards: usize,
    successes: usize,
    strike_consumer: Option<u32>,
}

// SimC's explicit deck parameters, not the talents' unused displayed chances.
const PROCS: &[SpendingProc] = &[
    SpendingProc {
        talent: 454009,
        buff: 454015,
        cards: 100,
        successes: 2,
        // Tempest's existing lightning/Thorim hooks order its consumption before spending.
        strike_consumer: None,
    },
    SpendingProc {
        talent: 1262713,
        buff: 1262830,
        cards: 250,
        successes: 5,
        strike_consumer: Some(187874),
    },
];

#[derive(Default)]
struct SpacedDeck {
    positions: Vec<usize>,
    cursor: usize,
}

impl SpacedDeck {
    fn draw(&mut self, proc: &SpendingProc) -> bool {
        if self.positions.is_empty() || self.cursor == proc.cards {
            let previous_distance = self
                .positions
                .iter()
                .max()
                .map_or(MAX_DRAW + 1, |last| proc.cards - last);
            self.positions = shuffle_successes(proc, previous_distance);
            self.cursor = 0;
        }
        let success = self.positions.contains(&self.cursor);
        self.cursor += 1;
        success
    }
}

fn shuffle_successes(proc: &SpendingProc, previous_distance: usize) -> Vec<usize> {
    let mut positions = Vec::with_capacity(proc.successes);
    for _ in 0..proc.successes {
        positions.push(shuffle_success_position(
            proc.cards,
            previous_distance,
            &positions,
        ));
    }
    positions
}

fn shuffle_success_position(cards: usize, previous_distance: usize, placed: &[usize]) -> usize {
    // SimC's rejection shuffle preserves spacing within AND across decks, with100 usable attempts.
    for _ in 0..100 {
        let position = crate::random::thread_rng().gen_range(0..cards);
        let separated = placed.iter().all(|old| old.abs_diff(position) > MAX_DRAW);
        if previous_distance + position > MAX_DRAW && separated {
            return position;
        }
    }
    panic!("MW deck could not place a spaced success after100 native shuffle attempts");
}

#[derive(Component, Default)]
struct SpendingDecks(BTreeMap<u32, SpacedDeck>);

pub(super) fn grant_for_spending(world: &mut World, caster: Entity, spent: u8) {
    if spent == 0 {
        return;
    }
    let selected: Vec<&SpendingProc> = PROCS
        .iter()
        .filter(|proc| {
            world
                .get::<KnownSpells>(caster)
                .is_some_and(|known| known.contains(proc.talent))
        })
        .collect();
    if selected.is_empty() {
        return;
    }
    if world.get::<SpendingDecks>(caster).is_none() {
        world.entity_mut(caster).insert(SpendingDecks::default());
    }
    for proc in selected {
        if draw_spent_stacks(world, caster, proc, spent) {
            spell_cast::cast_triggered(world, caster, proc.buff, Some(caster), 1);
        }
    }
}

fn draw_spent_stacks(world: &mut World, caster: Entity, proc: &SpendingProc, spent: u8) -> bool {
    let mut decks = world.get_mut::<SpendingDecks>(caster).unwrap();
    let deck = decks.0.entry(proc.talent).or_default();
    let successes = (0..spent).filter(|_| deck.draw(proc)).count();
    assert!(
        successes <= 1,
        "a native MW spend cannot draw two successes for talent{}",
        proc.talent
    );
    successes == 1
}

pub(super) fn consume_for_strike(world: &mut World, ctx: &EffectCtx) {
    // SimC6107: after the landed root's Thorim invocation, before Unleashed scheduling.
    for proc in PROCS
        .iter()
        .filter(|proc| proc.strike_consumer == Some(ctx.spell.id))
    {
        let instance = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == proc.buff && aura.caster == ctx.caster)
                .map(|aura| aura.instance_id)
        });
        if let Some(instance) = instance {
            auras::spend_stack(world, ctx.caster, instance);
        }
    }
}

#[cfg(test)]
fn assert_deck_spacing(proc: &SpendingProc) {
    let _seed = crate::random::seed_scope(70448);
    let mut deck = SpacedDeck::default();
    let draws: Vec<bool> = (0..proc.cards * 20).map(|_| deck.draw(proc)).collect();
    for cycle in draws.chunks_exact(proc.cards) {
        assert_eq!(cycle.iter().filter(|draw| **draw).count(), proc.successes);
    }
    for adjacent in draws.windows(MAX_DRAW + 1) {
        assert!(
            adjacent.iter().filter(|draw| **draw).count() <= 1,
            "successful cards stay more than10 apart, including across deck boundaries"
        );
    }
}

#[test]
fn enhancement_tempest_deck_preserves_success_count_and_spacing_across_refills() {
    assert_deck_spacing(&PROCS[0]);
}

#[test]
fn enhancement_storm_unleashed_deck_preserves_success_count_and_spacing_across_refills() {
    assert_deck_spacing(&PROCS[1]);
}
