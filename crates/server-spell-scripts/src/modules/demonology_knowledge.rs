//! Demonic Knowledge grants Core from the pinned SimC shuffled deck.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::random::seq::SliceRandom;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 386185;
const HAND_OF_GULDAN: u32 = 105174;
const CORE: u32 = 264173;

#[derive(Component, Default)]
struct KnowledgeDeck {
    rank: u32,
    cards: Vec<bool>,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterForegroundSpecial(draw)];

fn draw(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != HAND_OF_GULDAN {
        return;
    }
    let Some(rank) = world
        .get::<TraitLoadout>(caster)
        .and_then(|traits| traits.talent_rank(TALENT))
    else {
        return;
    };
    assert!(
        (1..=2).contains(&rank),
        "native Demonic Knowledge rank must be 1 or 2"
    );
    let mut deck = world
        .entity_mut(caster)
        .take::<KnowledgeDeck>()
        .unwrap_or_default();
    if deck.rank != rank || deck.cards.is_empty() {
        // SimC76f8118 sc_warlock_init.cpp1294–1305: rank1 has 6 winning
        // cards, rank2 12, in an 80-card deck shuffled after exhaustion.
        deck.rank = rank;
        deck.cards = vec![false; 80];
        deck.cards[..(rank as usize * 6)].fill(true);
        deck.cards.shuffle(&mut combat::random::thread_rng());
    }
    let won = deck.cards.pop().expect("refilled Demonic Knowledge deck");
    world.entity_mut(caster).insert(deck);
    if won {
        spell_cast::cast_triggered(world, caster, CORE, Some(caster), 1);
    }
}
