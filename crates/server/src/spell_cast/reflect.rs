//! Spell reflection, TrinityCore `WorldObject::SpellHitResult` (Object.cpp:1947-1978)
//! for a spell that can be reflected (`Spell::m_canReflect`, Spell.cpp:588-593): each
//! unit target rolls its REFLECT_SPELLS(_SCHOOL) chance when the spell launches
//! (`Spell::AddUnitTarget`, Spell.cpp:2479-2545). A reflected spell resolves on its
//! caster instead, after 1.5 × the target's hit delay, and the reflecting aura procs
//! (`PROC_HIT_REFLECT`) when the missile reaches the target (`ProcReflectDelayed`); a
//! spell with `SPELL_ATTR7_REFLECTION_ONLY_DEFENDS` is deflected (no effect) instead.
//! The reflected copy cannot reflect again, and its immunity check against the caster
//! (`CheckTarget`) is not modelled.

use crate::random as rand;
use bevy::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

use super::{attributes, targets};
use crate::auras::procs::{self, AuraEvent};
use crate::creature_ai::CreatureState;
use crate::spell_info::{SpellInfo, SpellStore};

/// `SPELL_ATTR0_IS_ABILITY` (SharedDefines.h:440).
const SPELL_ATTR0_IS_ABILITY: u32 = 0x0000_0010;
/// `SPELL_ATTR0_NO_IMMUNITIES` (SharedDefines.h:465).
const SPELL_ATTR0_NO_IMMUNITIES: u32 = 0x2000_0000;
/// `SPELL_ATTR1_NO_REFLECTION` (SharedDefines.h:480).
const SPELL_ATTR1_NO_REFLECTION: u32 = 0x0000_0080;
/// `SPELL_ATTR7_ALLOW_SPELL_REFLECTION` (SharedDefines.h:695).
const SPELL_ATTR7_ALLOW_SPELL_REFLECTION: u32 = 0x0000_0001;
/// `SPELL_ATTR7_REFLECTION_ONLY_DEFENDS` (SharedDefines.h:724).
const SPELL_ATTR7_REFLECTION_ONLY_DEFENDS: u32 = 0x2000_0000;
/// `SPELL_DAMAGE_CLASS_MAGIC` (`SpellCategories.DefenseType`).
const DEFENSE_MAGIC: u32 = 1;

/// What a unit target's reflection roll did to the spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reflection {
    /// `SPELL_MISS_REFLECT`: the spell turns on its caster.
    Reflect,
    /// `SPELL_MISS_DEFLECT`: the spell is stopped.
    Deflect,
}

/// `Spell::m_canReflect`: a magic spell that is not an ability, or one that allows
/// reflection, without `NO_REFLECTION` or `NO_IMMUNITIES`, and not passive.
fn can_reflect(spell: &SpellInfo) -> bool {
    let magic = spell
        .categories
        .as_ref()
        .is_some_and(|categories| categories.defense_type == DEFENSE_MAGIC);
    ((magic && attributes(spell, 0) & SPELL_ATTR0_IS_ABILITY == 0)
        || attributes(spell, 7) & SPELL_ATTR7_ALLOW_SPELL_REFLECTION != 0)
        && attributes(spell, 1) & SPELL_ATTR1_NO_REFLECTION == 0
        && attributes(spell, 0) & SPELL_ATTR0_NO_IMMUNITIES == 0
        && !crate::auras::is_passive(spell)
}

/// `SpellHitResult` up to its reflect roll for `target`: a positive spell at a
/// non-hostile unit, the caster itself and an evading creature never reflect.
pub(super) fn roll(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    target: Entity,
) -> Option<Reflection> {
    if caster == target || !can_reflect(spell) {
        return None;
    }
    let positive = world
        .get_resource::<SpellStore>()
        .is_some_and(|store| crate::spell_info::positivity::negative_effects(store, spell) == 0);
    if positive && !targets::is_hostile(world, caster, target) {
        return None;
    }
    if world.get::<CreatureState>(target) == Some(&CreatureState::Returning) {
        return None;
    }
    let chance = crate::auras::reflect_chance(world, target, spell.school_mask());
    if chance <= 0.0 || rand::random::<f32>() * 100.0 >= chance {
        return None;
    }
    Some(
        if attributes(spell, 7) & SPELL_ATTR7_REFLECTION_ONLY_DEFENDS != 0 {
            Reflection::Deflect
        } else {
            Reflection::Reflect
        },
    )
}

/// The Reflect or Deflect miss of `target` in the combat log, sent with the launch
/// (`SMSG_SPELL_GO` miss status).
pub(super) fn log_miss(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Entity,
    reflection: Reflection,
) {
    let kind = match reflection {
        Reflection::Reflect => MissKind::Reflect,
        Reflection::Deflect => MissKind::Deflect,
    };
    super::log_event(
        world,
        CombatLogEvent {
            source: Some(caster.to_bits()),
            target: Some(target.to_bits()),
            spell_id: Some(spell.id),
            school_mask: spell.school_mask(),
            amount: 0,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            extra_spell_id: None,
            timestamp_unix_ms: 0,
            kind: CombatLogKind::Miss(kind),
        },
    );
}

/// `ProcReflectDelayed::Execute` (Spell.cpp:2417-2431): the reflector takes a harmful
/// spell with `PROC_HIT_REFLECT`, so its reflect aura procs and spends a charge.
pub(super) fn proc_reflector(world: &mut World, caster: Entity, reflector: Entity) {
    if world.get_entity(caster).is_err() || world.get_entity(reflector).is_err() {
        return;
    }
    procs::push_event(
        world,
        AuraEvent {
            actor: caster,
            target: Some(reflector),
            actor_mask: [0; 2],
            target_mask: [
                procs::PROC_TAKE_HARMFUL_SPELL | procs::PROC_TAKE_HARMFUL_ABILITY,
                0,
            ],
            spell_id: None,
            can_proc: true,
            periodic: false,
            damage: 0.0,
            reflected: true,
        },
    );
}
