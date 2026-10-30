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
    SpellScriptHook::SummonRemoved(remove_storm_wind_gust),
    SpellScriptHook::TargetDamageMultiplier(stormfury_multiplier),
];

const ASCENDANCE: u32 = 1219480;
const STORMKEEPER: u32 = 191634;
const CALL_OF_FIRE: u32 = 378255;
const FURY_OF_THE_STORMS: u32 = 191717;
const PRIMAL_ELEMENTALIST: u32 = 117013;
const GREATER_FIRE: (u32, u32) = (188592, 95061);
const GREATER_STORM: (u32, u32) = (157299, 77936);
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
    let primal = selected(world, owner, PRIMAL_ELEMENTALIST);
    let elemental = match spell.id {
        ASCENDANCE if selected(world, owner, CALL_OF_FIRE) => {
            if primal { PRIMAL_FIRE } else { GREATER_FIRE }
        }
        STORMKEEPER if selected(world, owner, FURY_OF_THE_STORMS) => {
            if primal { PRIMAL_STORM } else { GREATER_STORM }
        }
        _ => return,
    };
    summon_elemental(world, owner, elemental);
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
    if let Some(expiry) = world
        .get_mut::<SummonedCreature>(actor)
        .unwrap()
        .expires_at
        .as_mut()
    {
        *expiry += f64::from(milliseconds) / 1000.0;
    }
    combat::auras::extend_owned_duration(world, owner, owner, &[root], milliseconds);
    spell_cast::guardian::reset_action_cooldowns(world, actor);
}

// SimC76f8118 sc_shaman.cpp4330–4334 expires Wind Gust when the
// actual Storm Elemental is dismissed, including early removal.
fn remove_storm_wind_gust(world: &mut World, actor: Entity) {
    let owner = world.get::<SummonedCreature>(actor).and_then(|summon| {
        (summon.spell_id == PRIMAL_STORM.0
            && world
                .get::<Npc>(actor)
                .is_some_and(|npc| npc.template_id == PRIMAL_STORM.1))
        .then_some(summon.owner)
    });
    let Some(owner) = owner else { return };
    combat::auras::remove_auras(
        world,
        owner,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 263806 && aura.caster == owner,
    );
}

fn stormfury_multiplier(world: &World, actor: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != 269005
        || world
            .get::<Npc>(actor)
            .is_none_or(|npc| npc.template_id != PRIMAL_STORM.1)
    {
        return 1.0;
    }
    let target = world
        .get::<combat::networking::CombatTarget>(actor)
        .and_then(|target| target.0)
        .expect("active Stormfury retains its primary target");
    let ticks = combat::auras::paid_periodic_ticks(world, actor, target, STORMFURY);
    let percent = spell_cast::scripts::script_effect_value(world, actor, STORMFURY, 1);
    // SimC76f8118 sc_shaman.cpp4160–4184: the child uses the parent channel's
    // current scheduled tick, shared by all recipients of that tick.
    (1.0 + percent / 100.0).powf(ticks as f32)
}

fn action_eligible(world: &World, actor: Entity, spell: u32, target: Entity) -> bool {
    match spell {
        // SimC76f8118 sc_shaman.cpp4275–4285: stormfury,if=buff.call_lightning.remains>=10.
        STORMFURY => {
            aura_remains(world, actor, actor, CALL_LIGHTNING)
                >= STORMFURY_MIN_CALL_LIGHTNING_SECONDS
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
