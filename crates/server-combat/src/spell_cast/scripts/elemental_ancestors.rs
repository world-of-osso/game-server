//! SimC76f8118 sc_shaman.cpp4342–4441,11579–11598,12413–12432.
//! Native445624 creates the real actor; native child spells own their damage.
use bevy::prelude::*;
use shared::components::Npc;

use super::super::summon::SummonedCreature;
use crate::class_progression::{ActiveSpec, TraitLoadout};
use crate::spell_info::SpellInfo;

const CALL: u32 = 443450;
const SUMMON: u32 = 445624;
const BUFF: u32 = 447244;
const ANCESTOR: u32 = 221177;
const FINAL_CALLING: u32 = 443446;
const BLAST: u32 = 465717;

fn selected(world: &World, owner: Entity, talent: u32) -> bool {
    world
        .get::<ActiveSpec>(owner)
        .is_some_and(|spec| spec.0 == 262)
        && world.get::<TraitLoadout>(owner).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == talent)
        })
}

pub(in crate::spell_cast) fn owner(world: &World, actor: Entity) -> Option<Entity> {
    let summon = world.get::<SummonedCreature>(actor)?;
    (summon.spell_id == SUMMON && world.get::<Npc>(actor)?.template_id == ANCESTOR)
        .then_some(summon.owner)
}

fn actors(world: &mut World, owner: Entity) -> Vec<Entity> {
    let now = super::super::now(world);
    world
        .query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(world)
        .filter(|(_, summon, npc)| {
            summon.owner == owner
                && summon.spell_id == SUMMON
                && npc.template_id == ANCESTOR
                && summon.expires_at.is_some_and(|expiry| expiry > now)
        })
        .map(|(actor, _, _)| actor)
        .collect()
}

fn summon(world: &mut World, owner: Entity, depth: u8) {
    let before = actors(world, owner).len();
    if !super::super::try_cast_triggered(world, owner, SUMMON, Some(owner), depth + 1) {
        warn!(?owner, "Native Ancestor summon refused");
        return;
    }
    if actors(world, owner).len() > before {
        super::super::cast_triggered(world, owner, BUFF, Some(owner), depth + 1);
    }
}

// SimC sc_shaman.cpp6358,7103,7374,7636,7856,8213,8358,8495,8567.
// A successful foreground cast commands every living Ancestor once.
const ECHOES: &[(u32, u32)] = &[
    (188443, 447425),
    (51505, 447419),
    (196840, 447419),
    (117014, 447419),
    (188196, 447419),
    (8042, 447419),
    (61882, 447425),
    (462620, 447425),
    (470411, 447419),
    (1219480, 447425),
];

pub(super) fn after_launch(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
    foreground: bool,
) {
    if !foreground || spell.cast_parent_owns_modifier_charges || !selected(world, caster, CALL) {
        return;
    }
    if let Some(&(_, child)) = ECHOES.iter().find(|rule| rule.0 == spell.id) {
        if let Some(target) = spell.cast_primary_target {
            for actor in actors(world, caster) {
                if !super::super::try_cast_triggered(world, actor, child, Some(target), depth + 1) {
                    warn!(?actor, child, "Ancestor echo refused");
                }
            }
        }
    }
    // sc_shaman.cpp8758/8163–8166: SK and selected Nature's Swiftness's
    // Ancestral replacement call a new ancestor after the owner's execution.
    if spell.id == 191634 || (spell.id == 443454 && selected(world, caster, 378081)) {
        summon(world, caster, depth);
    }
}

pub(super) fn before_launch(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell == BLAST {
        if let Some(owner) = owner(world, caster) {
            // sc_shaman.cpp4382: Final Calling grants the owner's Blast buff first.
            super::elemental::grant_blast_buff(world, owner, 120588, true, depth);
        }
    }
}

pub(in crate::spell_cast) fn on_expire(world: &mut World, actor: Entity) {
    let Some(owner) = owner(world, actor) else {
        return;
    };
    if selected(world, owner, FINAL_CALLING) {
        // sc_shaman.cpp4421–4433 chooses an actual valid enemy on natural expiry.
        use crate::random::seq::SliceRandom;
        let targets: Vec<_> = crate::combat_refs::active_opponents(world, owner)
            .filter(|enemy| super::super::targets::is_alive(world, *enemy))
            .collect();
        let target = targets.choose(&mut crate::random::thread_rng()).copied();
        if let Some(target) = target {
            super::super::cast_triggered(world, actor, BLAST, Some(target), 1);
        }
    }
}

// SimC76f8118 sc_shaman.cpp4355–4361: guaranteed crit also scales its
// action multiplier by composite crit chance (base1 plus inherited owner crit).
pub(super) fn lava_multiplier(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    let Some(owner) = owner(world, caster) else {
        return amount;
    };
    let crit = world
        .get::<crate::stats::CharacterStats>(owner)
        .expect("Ancestor owner stats")
        .crit_pct;
    amount * (1.0 + crit / 100.0)
}

// SimC76f8118 sc_shaman.cpp11468–11469 adds the three Ancestor spells
// to Elemental Fury's first effect; parse_effects.cpp1895 uses one-based indexes.
// Native60188 effect0 owns the bonus; do not fabricate family masks or amounts.
pub(in crate::spell_cast) fn modify_critical_bonus(
    world: &World,
    caster: Entity,
    spell: u32,
    bonus: f32,
) -> f32 {
    if !matches!(spell, 447419 | 447425 | BLAST) {
        return bonus;
    }
    let Some(owner) = owner(world, caster) else {
        return bonus;
    };
    if !selected(world, owner, 60188) {
        return bonus;
    }
    let effect = world
        .get::<crate::auras::ActiveAuras>(owner)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 60188 && aura.caster == owner)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0));
    effect.map_or(bonus, |effect| bonus * (1.0 + effect.amount / 100.0))
}

pub(in crate::spell_cast) fn on_despawn(world: &mut World, actor: Entity) {
    let Some(owner) = owner(world, actor) else {
        return;
    };
    let instance = world
        .get::<crate::auras::ActiveAuras>(owner)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BUFF && aura.caster == owner)
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        crate::auras::spend_stack(world, owner, instance);
    }
}
