//! SimC76f8118 sc_shaman.cpp556–629/11764/12173–12191/13057–13067:
//! real MW spending draws a spaced two-success100-card deck, at most once per spell.
use bevy::prelude::*;

use crate::class_progression::KnownSpells;
use crate::random::Rng;
use crate::spell_cast;

// These are SimC's behavioral deck parameters, not the unused talent chance or stale200-card comment.
const DECK_SIZE: usize = 100;
const MAX_DRAW: usize = 10;
const TEMPEST: u32 = 454009;
const TEMPEST_PROC: u32 = 454015;

#[derive(Component, Default)]
struct TempestDeck {
    positions: Option<[usize; 2]>,
    cursor: usize,
}

impl TempestDeck {
    fn draw(&mut self) -> bool {
        if self.positions.is_none() || self.cursor == DECK_SIZE {
            let previous_distance = self.positions.map_or(MAX_DRAW + 1, |positions| {
                DECK_SIZE - positions[0].max(positions[1])
            });
            let first = shuffle_success_position(previous_distance, &[]);
            let second = shuffle_success_position(previous_distance, &[first]);
            self.positions = Some([first, second]);
            self.cursor = 0;
        }
        let success = self
            .positions
            .expect("Tempest deck must be shuffled before drawing")
            .contains(&self.cursor);
        self.cursor += 1;
        success
    }
}

fn shuffle_success_position(previous_distance: usize, placed: &[usize]) -> usize {
    // SimC's rejection shuffle preserves spacing within AND across decks, with100 usable attempts.
    for _ in 0..100 {
        let position = crate::random::thread_rng().gen_range(0..DECK_SIZE);
        let separated = placed.iter().all(|old| old.abs_diff(position) > MAX_DRAW);
        if previous_distance + position > MAX_DRAW && separated {
            return position;
        }
    }
    panic!("Tempest deck could not place a spaced success after100 native shuffle attempts");
}

pub(super) fn grant_for_spending(world: &mut World, caster: Entity, spent: u8) {
    if spent == 0
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(TEMPEST))
    {
        return;
    }
    if world.get::<TempestDeck>(caster).is_none() {
        world.entity_mut(caster).insert(TempestDeck::default());
    }
    let successes = {
        let mut deck = world.get_mut::<TempestDeck>(caster).unwrap();
        (0..spent).filter(|_| deck.draw()).count()
    };
    assert!(
        successes <= 1,
        "a native MW spend cannot draw two Tempest successes"
    );
    if successes == 1 {
        spell_cast::cast_triggered(world, caster, TEMPEST_PROC, Some(caster), 1);
    }
}

#[test]
fn enhancement_tempest_deck_preserves_success_count_and_spacing_across_refills() {
    let _seed = crate::random::seed_scope(70448);
    let mut deck = TempestDeck::default();
    let draws: Vec<bool> = (0..2000).map(|_| deck.draw()).collect();
    for cycle in draws.chunks_exact(100) {
        assert_eq!(cycle.iter().filter(|draw| **draw).count(), 2);
    }
    for adjacent in draws.windows(11) {
        assert!(
            adjacent.iter().filter(|draw| **draw).count() <= 1,
            "successful cards stay more than10 apart, including across deck boundaries"
        );
    }
}
