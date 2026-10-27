//! Native Ancient Fellowship draws once per naturally expiring Ancestor.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::class_progression::{ActiveSpec, TraitLoadout};
use combat::random::seq::SliceRandom;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SummonExpired(on_expiry)];
const FELLOWSHIP: u32 = 443423;

#[derive(Component, Default)]
struct FellowshipDeck(Vec<bool>);

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 262)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|entry| entry.spell_id == FELLOWSHIP)
        })
}

fn draw(world: &mut World, caster: Entity) -> bool {
    if world.get::<FellowshipDeck>(caster).is_none() {
        world.entity_mut(caster).insert(FellowshipDeck::default());
    }
    if world.get::<FellowshipDeck>(caster).unwrap().0.is_empty() {
        let aura = world
            .get::<ActiveAuras>(caster)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == FELLOWSHIP && aura.caster == caster)
            })
            .expect("selected native Ancient Fellowship aura");
        let amount = |index| {
            aura.effects
                .iter()
                .find(|effect| effect.index == index)
                .expect("native Fellowship deck effect")
                .amount as usize
        };
        let (total, successes) = (amount(1), amount(2));
        assert!(
            successes > 0 && total >= successes,
            "native Fellowship deck counts"
        );
        let mut deck = vec![false; total];
        deck[..successes].fill(true);
        deck.shuffle(&mut combat::random::thread_rng());
        world.get_mut::<FellowshipDeck>(caster).unwrap().0 = deck;
    }
    world
        .get_mut::<FellowshipDeck>(caster)
        .unwrap()
        .0
        .pop()
        .expect("refilled native Fellowship deck")
}

fn on_expiry(world: &mut World, actor: Entity) {
    let Some(owner) = spell_cast::scripts::ancestor_owner(world, actor) else {
        return;
    };
    // SimC76f8118 sc_shaman.cpp4437–4439/13016–13023: natural expiry draws once.
    if selected(world, owner) && draw(world, owner) {
        spell_cast::scripts::summon_ancestor(world, owner, 0);
    }
}
