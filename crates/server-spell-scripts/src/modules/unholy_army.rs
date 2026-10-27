//! SimC76f8118 sc_death_knight.cpp8805–8836/8942–9010: Army summons its real lesser ghouls over time.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const ARMY: u32 = 42650;
const LESSER_GHOUL: u32 = 275430;
// SimC76f8118 sc_death_knight.cpp15401–15402: Army uses its own 30s root.
const ARMY_GHOUL: u32 = 1294026;
const SUMMON_INTERVAL: f64 = 0.5;

#[derive(Component)]
struct PendingArmy {
    remaining: u32,
    next_summon: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::AfterCastUpdate(update),
    SpellScriptHook::SummonInitialized(engage_lesser_ghoul),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        ARMY => Some(schedule_army),
        1233448 => Some(summon_magus),
        _ => None,
    }
}

fn summon_magus(world: &mut World, caster: Entity, _: &SpellInfo) {
    if combat::auras::has_aura(world, caster, 390196)
        && !spell_cast::try_cast_triggered(world, caster, 317776, Some(caster), 1)
    {
        warn!(?caster, "native Magus of the Dead summon refused");
    }
}

fn schedule_army(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(252) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp9019–9035: Army raises a Magus and
    // starts Forbidden Knowledge, whose native linked auras replace RP spenders.
    summon_magus(world, caster, spell);
    if combat::auras::has_aura(world, caster, 1242158) {
        combat::auras::apply_aura(world, &combat::auras::AuraApplication {
            caster, target: caster, spell_id: 1242223, effect_mask: 3,
            source: combat::auras::AuraSource::Cast,
        });
    }
    let count = spell_cast::scripts::script_effect_value(world, caster, ARMY, 0) as u32;
    assert!(count > 0, "native Army ghoul count must be positive");
    let next_summon = combat::auras::aura_now(world) + SUMMON_INTERVAL;
    world.entity_mut(caster).insert(PendingArmy {
        remaining: count,
        next_summon,
    });
}

fn engage_lesser_ghoul(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<spell_cast::summon::SummonedCreature>(actor) else {
        return;
    };
    if !matches!(summon.spell_id, LESSER_GHOUL | ARMY_GHOUL) {
        return;
    }
    let owner = summon.owner;
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0)
    else {
        warn!(?owner, "Army lesser ghoul has no selected target");
        return;
    };
    // SimC76f8118 sc_death_knight.cpp3155–3211: the real lesser ghoul
    // engages the owner's target using the shared summon melee chase.
    spell_cast::summon::start_melee_attack(world, actor, target);
}

fn update(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let mut due = Vec::new();
    for (caster, mut pending) in world.query::<(Entity, &mut PendingArmy)>().iter_mut(world) {
        while pending.remaining > 0 && now >= pending.next_summon {
            due.push(caster);
            pending.remaining -= 1;
            pending.next_summon += SUMMON_INTERVAL;
        }
    }
    for caster in due {
        if !spell_cast::try_cast_triggered(world, caster, ARMY_GHOUL, Some(caster), 1) {
            warn!(?caster, "Native Army lesser-ghoul summon refused");
        }
    }
}
