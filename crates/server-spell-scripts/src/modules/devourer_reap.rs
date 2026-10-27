//! Native Reap fragment consumption and source-defined delayed damage.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::soul_fragments::{self, FragmentFilter};
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

const REAP: u32 = 1226019;
const REAP_DAMAGE: u32 = 1225823;
const SHATTERED_SOULS: u32 = 1227619;
const MOMENT_OF_CRAVING: u32 = 1238495;
const SOULSHAPER: u32 = 1238739;

struct PendingReap {
    at: f64,
    target: Entity,
    consumed: usize,
}

#[derive(Component, Default)]
struct PendingReaps(Vec<PendingReap>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeLaunch(after_launch),
    SpellScriptHook::AfterNativeCastUpdate(update),
];

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != REAP {
        return;
    }
    let target = spell
        .cast_primary_target
        .expect("native Reap retains its target");
    let mut maximum = script_effect_value(world, caster, SHATTERED_SOULS, 1);
    if combat::auras::has_aura(world, caster, MOMENT_OF_CRAVING) {
        maximum += script_effect_value(world, caster, MOMENT_OF_CRAVING, 0);
    }
    // SimC76f8118 sc_demon_hunter.cpp6008–6018,6081–6107 consumes
    // active lesser fragments on execution; damage follows after 220ms.
    let consumed = soul_fragments::consume_active(
        world,
        caster,
        FragmentFilter::Lesser,
        maximum.max(0.0) as usize,
        false,
    )
    .expect("native Reap fragment consumption");
    let pending = PendingReap {
        at: spell_cast::now(world) + 0.220,
        target,
        consumed,
    };
    world
        .entity_mut(caster)
        .entry::<PendingReaps>()
        .or_default()
        .into_mut()
        .0
        .push(pending);
}

fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let casters: Vec<_> = world
        .query_filtered::<Entity, With<PendingReaps>>()
        .iter(world)
        .collect();
    for caster in casters {
        let pending = {
            let mut queue = world
                .get_mut::<PendingReaps>(caster)
                .expect("pending Reap queue");
            let (ready, later): (Vec<_>, Vec<_>) = std::mem::take(&mut queue.0)
                .into_iter()
                .partition(|item| item.at <= now);
            queue.0 = later;
            ready
        };
        for item in pending {
            execute_damage(world, caster, item);
        }
    }
}

fn execute_damage(world: &mut World, caster: Entity, pending: PendingReap) {
    let Some(mut damage) = spell_cast::load_triggered_spell(world, REAP_DAMAGE, 1) else {
        error!(?caster, "native Reap damage 1225823 missing");
        return;
    };
    if world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(SOULSHAPER))
    {
        let percent = script_effect_value(world, caster, SOULSHAPER, 0);
        damage.cast_damage_multiplier = Some(1.0 + pending.consumed as f32 * percent / 100.0);
    }
    if !spell_cast::launch_triggered_spell(world, caster, &damage, Some(pending.target), 1) {
        warn!(?caster, target = ?pending.target, "native Reap damage was refused");
    }
}
