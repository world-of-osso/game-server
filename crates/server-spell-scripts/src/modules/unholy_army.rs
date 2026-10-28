//! SimC76f8118 sc_death_knight.cpp8805–8836/8942–9010: Army summons its real lesser ghouls over time.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const ARMY: u32 = 42650;
const LESSER_GHOUL: u32 = 275430;
// SimC76f8118 sc_death_knight.cpp15401–15402: Army uses its own 30s root.
const ARMY_GHOUL: u32 = 1294026;
const SUMMON_INTERVAL: f64 = 0.5;
const APOCALYPSE_NOW: u32 = 444040;
const HORSEMEN: &[u32] = &[444248, 444251, 444252, 444254];

#[derive(Component)]
struct PendingArmy {
    remaining: u32,
    next_summon: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::AfterCastUpdate(update),
    SpellScriptHook::SummonInitialized(engage_melee_summon),
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
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster,
                target: caster,
                spell_id: 1242223,
                effect_mask: 3,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
    let count = spell_cast::scripts::script_effect_value(world, caster, ARMY, 0) as u32;
    assert!(count > 0, "native Army ghoul count must be positive");
    let next_summon = combat::auras::aura_now(world) + SUMMON_INTERVAL;
    world.entity_mut(caster).insert(PendingArmy {
        remaining: count,
        next_summon,
    });
    summon_apocalypse_now_riders(world, caster);
}

fn summon_apocalypse_now_riders(world: &mut World, caster: Entity) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == APOCALYPSE_NOW)
    }) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp9027–9030/13651–13689: Army
    // summons all four or extends existing riders by the selected duration.
    let duration = f64::from(spell_cast::scripts::script_effect_value(
        world,
        caster,
        APOCALYPSE_NOW,
        1,
    )) / 1000.0;
    assert!(
        duration > 0.0,
        "Apocalypse Now requires a positive native duration"
    );
    for &spell in HORSEMEN {
        let now = combat::auras::aura_now(world);
        let existing = world
            .query::<(Entity, &spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .find_map(|(actor, summon)| {
                (summon.owner == caster
                    && summon.spell_id == spell
                    && summon.expires_at.is_some_and(|expiry| expiry > now))
                .then_some(actor)
            });
        if let Some(actor) = existing {
            let mut summon = world
                .get_mut::<spell_cast::summon::SummonedCreature>(actor)
                .unwrap();
            *summon.expires_at.as_mut().unwrap() += duration;
            continue;
        }
        if !spell_cast::try_cast_triggered(world, caster, spell, Some(caster), 1) {
            warn!(?caster, spell, "Native Horseman summon refused");
            continue;
        }
        let actor = world
            .query::<(Entity, &spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .find_map(|(actor, summon)| {
                (summon.owner == caster && summon.spell_id == spell).then_some(actor)
            })
            .expect("successful Horseman summon must create an actor");
        world
            .get_mut::<spell_cast::summon::SummonedCreature>(actor)
            .unwrap()
            .expires_at = Some(now + duration);
    }
}

fn engage_melee_summon(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<spell_cast::summon::SummonedCreature>(actor) else {
        return;
    };
    if !matches!(summon.spell_id, LESSER_GHOUL | ARMY_GHOUL) && !HORSEMEN.contains(&summon.spell_id)
    {
        return;
    }
    let owner = summon.owner;
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0)
    else {
        warn!(?owner, "Unholy melee summon has no selected target");
        return;
    };
    // SimC76f8118 sc_death_knight.cpp3155–3211/4849–4855: the actual
    // lesser ghoul or Horseman uses the shared summon melee chase.
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
