//! Out-of-combat raid buffs a class follower casts on its party, as players do.
//!
//! SimC76f8118 sim.cpp:3538–3543: `optimal_raid` sets one override per buff
//! below, and the overrides, not the pinned APLs, give the raid these buffs.
//! The APLs therefore omit them (Mage's precombat `arcane_intellect` excepted),
//! and this step casts each buff a party member lacks.
use bevy::prelude::*;
use server::spell_cast::CastFailure;
use shared::components::{CombatStatus, Position};

/// Each buff with its SimC override name. This step alone restores them; a parity
/// reference sets `override.<name>=1` for exactly the buffs its composition provides.
pub const PARTY_BUFFS: [(u32, &str); 6] = [
    (1459, "arcane_intellect"),
    (6673, "battle_shout"),
    (1126, "mark_of_the_wild"),
    (21562, "power_word_fortitude"),
    (462854, "skyfury"),
    (364342, "blessing_of_the_bronze"),
];

pub fn is_party_buff(spell: u32) -> bool {
    PARTY_BUFFS.iter().any(|&(buff, _)| buff == spell)
}

/// The party buffs any of `units` knows: the set this composition provides.
pub fn provided_buffs(world: &World, units: &[Entity]) -> Vec<(u32, &'static str)> {
    PARTY_BUFFS
        .into_iter()
        .filter(|&(spell, _)| units.iter().any(|&unit| knows(world, unit, spell)))
        .collect()
}

/// `unit` holds one of the auras the party buff `spell` grants.
pub fn holds_buff(world: &World, unit: Entity, spell: u32) -> Result<bool, String> {
    Ok(read_buff(world, spell)?
        .results
        .iter()
        .any(|&aura| server::auras::has_aura(world, unit, aura)))
}

/// The first buff `caster` provides that a recipient it reaches lacks.
pub fn own_lost_buff(world: &mut World, caster: Entity) -> Result<Option<u32>, String> {
    let party = crate::party_of(world, caster);
    for (spell, _) in PARTY_BUFFS {
        if knows(world, caster, spell) && lacking(world, caster, &party, &read_buff(world, spell)?)
        {
            return Ok(Some(spell));
        }
    }
    Ok(None)
}

/// Cast at most one lost party buff; true when `actor` started a cast.
/// The caller has already checked that `actor` is off its GCD and not casting.
pub(crate) fn cast_lost_party_buff(world: &mut World, actor: Entity) -> Result<bool, String> {
    if in_combat(world, actor) {
        return Ok(false);
    }
    let party = crate::party_of(world, actor);
    for (spell, _) in PARTY_BUFFS {
        if chosen_caster(world, actor, &party, spell) != Some(actor) {
            continue;
        }
        if !lacking(world, actor, &party, &read_buff(world, spell)?) {
            continue;
        }
        return match server::spell_cast::creature_cast(world, actor, spell, Some(actor), false) {
            Ok(()) => Ok(true),
            Err(
                CastFailure::InProgress
                | CastFailure::OnGlobalCooldown
                | CastFailure::Moving
                | CastFailure::NotEnoughPower(_)
                | CastFailure::CrowdControlled(_),
            ) => Ok(false),
            Err(error) => Err(format!("party buff spell{spell}: {error:?}")),
        };
    }
    Ok(false)
}

/// The auras a buff grants each recipient and the radius they reach.
struct PartyBuff {
    results: Vec<u32>,
    radius: f32,
}

/// The root aura, or for Blessing of the Bronze the class auras its effects trigger.
fn read_buff(world: &World, spell: u32) -> Result<PartyBuff, String> {
    let store = world.resource::<server::spell_info::SpellStore>();
    let root = store
        .get(spell)
        .ok_or_else(|| format!("party buff spell{spell} missing"))?;
    let mut results = Vec::new();
    let mut radius = 0.0f32;
    if server::auras::aura_effect_mask(root) != 0 {
        results.push(spell);
    }
    for effect in &root.effects {
        radius = radius.max(server::spell_cast::targets::effect_radius(effect));
        if effect.effect != SPELL_EFFECT_TRIGGER_SPELL {
            continue;
        }
        if let Some(child) = store.get(effect.trigger_spell)
            && server::auras::aura_effect_mask(child) != 0
        {
            results.push(child.id);
            for effect in &child.effects {
                radius = radius.max(server::spell_cast::targets::effect_radius(effect));
            }
        }
    }
    if results.is_empty() || radius <= 0.0 {
        return Err(format!("party buff spell{spell} grants no area aura"));
    }
    Ok(PartyBuff { results, radius })
}

const SPELL_EFFECT_TRIGGER_SPELL: u32 = 64;

/// One caster per buff: the lowest entity able to cast it among the units sharing
/// `actor`'s group, so an ungrouped follower buffs itself.
fn chosen_caster(world: &World, actor: Entity, party: &[Entity], spell: u32) -> Option<Entity> {
    std::iter::once(actor)
        .chain(party.iter().copied())
        .filter(|&unit| {
            server::spell_cast::targets::shares_raid(world, actor, unit)
                && world
                    .get::<server::followers::AplControlled>(unit)
                    .is_some()
                && server::spell_cast::targets::is_alive(world, unit)
                && !in_combat(world, unit)
                && knows(world, unit, spell)
        })
        .min_by_key(|unit| unit.to_bits())
}

/// A living recipient in range of `caster` has none of the buff's auras. Recipients
/// are the party units the raid-buff targets reach: the caster and its group.
fn lacking(world: &World, caster: Entity, party: &[Entity], buff: &PartyBuff) -> bool {
    let Some(origin) = world.get::<Position>(caster) else {
        return false;
    };
    std::iter::once(&caster).chain(party).any(|&member| {
        server::spell_cast::targets::shares_raid(world, caster, member)
            && server::spell_cast::targets::is_alive(world, member)
            && server::spell_cast::targets::same_map(world, caster, member)
            && world.get::<Position>(member).is_some_and(|at| {
                Vec3::new(at.x, at.y, at.z).distance(Vec3::new(origin.x, origin.y, origin.z))
                    <= buff.radius
            })
            && !buff
                .results
                .iter()
                .any(|&aura| server::auras::has_aura(world, member, aura))
    })
}

fn knows(world: &World, unit: Entity, spell: u32) -> bool {
    world
        .get::<server::class_progression::KnownSpells>(unit)
        .is_some_and(|known| known.contains(spell))
}

fn in_combat(world: &World, unit: Entity) -> bool {
    world
        .get::<CombatStatus>(unit)
        .is_some_and(|status| status.0)
}
