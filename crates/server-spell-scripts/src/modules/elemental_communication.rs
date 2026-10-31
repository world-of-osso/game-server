//! Native Routine Communication draws once per ordinary builder execution.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, SpellModOp, spell_mods};
use combat::class_progression::{ActiveSpec, TraitLoadout};
use combat::random::seq::SliceRandom;
use combat::spell_cast::{self, script_registry::SpellScriptHook, targets};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterElementalLaunch(draw_after_builder),
    // SimC76f8118 sc_shaman.cpp12726–12728: each ancestor buff stack expires independently.
    SpellScriptHook::AsynchronousAuras(&[447244]),
];

const COMMUNICATION: u32 = 443445;
const BUILDERS: &[u32] = &[188196, 188443, 51505, 188389];

#[derive(Component, Default)]
struct CommunicationDeck(Vec<bool>);

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 262)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == COMMUNICATION)
        })
}

fn read_native_counts(world: &World, caster: Entity) -> (usize, usize) {
    let now = spell_cast::now(world);
    let aura = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras.iter().find(|aura| {
                aura.spell_id == COMMUNICATION
                    && aura.caster == caster
                    && aura.expires_at.is_none_or(|at| at > now)
            })
        })
        .expect("selected native Routine Communication aura");
    let amount = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native Routine Communication deck effect")
            .amount as usize
    };
    // SimC76f8118 sc_shaman.cpp13025–13033: effect5 successes/effect6 total.
    let counts = (amount(4), amount(5));
    assert!(
        counts.0 > 0 && counts.1 >= counts.0,
        "native Communication deck counts"
    );
    counts
}

fn draw_native_deck(world: &mut World, caster: Entity) -> bool {
    if world.get::<CommunicationDeck>(caster).is_none() {
        world
            .entity_mut(caster)
            .insert(CommunicationDeck::default());
    }
    let refill = world.get::<CommunicationDeck>(caster).unwrap().0.is_empty();
    if refill {
        let (successes, total) = read_native_counts(world, caster);
        let mut cards = vec![false; total];
        cards[..successes].fill(true);
        // SimC76f8118 proc_rng.cpp149–161: reshuffle after each complete deck.
        cards.shuffle(&mut combat::random::thread_rng());
        world.get_mut::<CommunicationDeck>(caster).unwrap().0 = cards;
    }
    world
        .get_mut::<CommunicationDeck>(caster)
        .unwrap()
        .0
        .pop()
        .expect("refilled native Routine Communication deck")
}

fn draw_after_builder(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    // SimC76f8118 sc_shaman.cpp6654–6658/7260–7263/7436–7438/8473–8480:
    // normal Chain/Lava/Bolt/Flame Shock execute, not background children.
    if !foreground || spell.cast_parent_owns_modifier_charges || !selected(world, caster) {
        return;
    }
    if spell.id == super::voltaic_blaze::ROOT {
        draw_for_voltaic_targets(world, caster, spell);
    } else if BUILDERS.contains(&spell.id) {
        draw_and_summon(world, caster);
    }
}

fn draw_for_voltaic_targets(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let origin = targets::position(world, caster).expect("Voltaic caster retains position");
    let (_, maximum) = targets::spell_range(spell, true);
    let maximum = spell_mods::modify(world, caster, spell, SpellModOp::Range, maximum);
    let enemies = targets::area_units(world, caster, origin, maximum, targets::Relation::Enemy, 0);
    // SimC76f8118 sc_shaman.cpp10130–10135 draws for the full target list,
    // not the capped Voltaic damage recipients.
    for _ in enemies {
        draw_and_summon(world, caster);
    }
}

pub(super) fn draw_and_summon(world: &mut World, caster: Entity) {
    if selected(world, caster) && draw_native_deck(world, caster) {
        spell_cast::scripts::summon_ancestor(world, caster, 0);
    }
}
