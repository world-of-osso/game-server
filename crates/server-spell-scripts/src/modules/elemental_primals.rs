//! Native Primal Elementals: talent summons and pinned pet APL action conditions.
use bevy::prelude::*;
use shared::components::Npc;

use super::combat;
use combat::auras::ActiveAuras;
use combat::class_progression::{ActiveSpec, TraitLoadout};
use combat::spell_cast::{self, script_registry::SpellScriptHook, summon::SummonedCreature};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterElementalLaunch(summon_after_launch),
    SpellScriptHook::GuardianActionEligible(action_eligible),
];

const ASCENDANCE: u32 = 1219480;
const STORMKEEPER: u32 = 191634;
const CALL_OF_FIRE: u32 = 378255;
const FURY_OF_THE_STORMS: u32 = 191717;
const PRIMAL_ELEMENTALIST: u32 = 117013;
const PRIMAL_FIRE: (u32, u32) = (118291, 61029);
const PRIMAL_STORM: (u32, u32) = (157319, 77942);
const STORMFURY: u32 = 157375;
const CALL_LIGHTNING: u32 = 157348;
const IMMOLATE: u32 = 118297;
const STORMFURY_MIN_CALL_LIGHTNING_SECONDS: f64 = 10.0;

fn selected(world: &World, owner: Entity, talent: u32) -> bool {
    world
        .get::<ActiveSpec>(owner)
        .is_some_and(|spec| spec.0 == 262)
        && world.get::<TraitLoadout>(owner).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|entry| entry.spell_id == talent)
        })
}

fn summon_after_launch(world: &mut World, owner: Entity, spell: &SpellInfo, _foreground: bool) {
    // SimC76f8118 sc_shaman.cpp8638–8640: every Ascendance execution, background
    // variants included; 8752–8755: Stormkeeper. Primal variants need 117013.
    let elemental = match spell.id {
        ASCENDANCE if selected(world, owner, CALL_OF_FIRE) => PRIMAL_FIRE,
        STORMKEEPER if selected(world, owner, FURY_OF_THE_STORMS) => PRIMAL_STORM,
        _ => return,
    };
    if selected(world, owner, PRIMAL_ELEMENTALIST) {
        summon_elemental(world, owner, elemental);
    }
}

fn active_elemental(world: &mut World, owner: Entity, (root, entry): (u32, u32)) -> Option<Entity> {
    let now = spell_cast::now(world);
    world
        .query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(world)
        .find(|(_, summon, npc)| {
            summon.owner == owner
                && summon.spell_id == root
                && npc.template_id == entry
                && summon.expires_at.is_some_and(|at| at > now)
        })
        .map(|(actor, _, _)| actor)
}

/// SimC76f8118 sc_shaman.cpp11555–11570: extend an active elemental and the
/// owner's buff by the native duration and reset its action cooldowns,
/// otherwise summon the actual actor.
fn summon_elemental(world: &mut World, owner: Entity, elemental: (u32, u32)) {
    let root = elemental.0;
    let Some(actor) = active_elemental(world, owner, elemental) else {
        if !spell_cast::try_cast_triggered(world, owner, root, Some(owner), 1) {
            warn!(?owner, root, "Native Primal Elemental summon refused");
        }
        return;
    };
    let milliseconds = world
        .resource::<SpellStore>()
        .get(root)
        .and_then(|spell| spell.misc.as_ref()?.duration)
        .map(|duration| duration.duration_ms)
        .expect("native Primal Elemental summon duration");
    let milliseconds = u32::try_from(milliseconds).expect("finite native summon duration");
    if let Some(expiry) = world.get_mut::<SummonedCreature>(actor).unwrap().expires_at.as_mut() {
        *expiry += f64::from(milliseconds) / 1000.0;
    }
    combat::auras::extend_owned_duration(world, owner, owner, &[root], milliseconds);
    spell_cast::guardian::reset_action_cooldowns(world, actor);
}

fn action_eligible(world: &World, actor: Entity, spell: u32, target: Entity) -> bool {
    match spell {
        // SimC76f8118 sc_shaman.cpp4275–4285: stormfury,if=buff.call_lightning.remains>=10.
        STORMFURY => {
            aura_remains(world, actor, actor, CALL_LIGHTNING) >= STORMFURY_MIN_CALL_LIGHTNING_SECONDS
        }
        // sc_shaman.cpp4109–4117: immolate,target_if=!ticking.
        IMMOLATE => aura_remains(world, target, actor, IMMOLATE) <= 0.0,
        _ => true,
    }
}

fn aura_remains(world: &World, unit: Entity, caster: Entity, spell: u32) -> f64 {
    let now = spell_cast::now(world);
    world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == caster)
        })
        .and_then(|aura| aura.expires_at)
        .map_or(0.0, |at| (at - now).max(0.0))
}
