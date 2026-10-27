//! Native Reap and Cull fragment consumption and source-defined delayed damage.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::soul_fragments::{self, FragmentFilter};
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

const REAP: u32 = 1226019;
const REAP_DAMAGE: u32 = 1225823;
const CULL: u32 = 1245453;
const CULL_DAMAGE: u32 = 1245455;
const SHATTERED_SOULS: u32 = 1227619;
const MOMENT_OF_CRAVING: u32 = 1238495;
const SOULSHAPER: u32 = 1238739;

#[derive(Component, Default)]
struct SoulburstFailures(u32);

struct PendingReap {
    at: f64,
    target: Entity,
    damage: u32,
    consumed: usize,
}

#[derive(Component, Default)]
struct PendingReaps(Vec<PendingReap>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeLaunch(after_launch),
    SpellScriptHook::AfterNativeCastUpdate(update),
];

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_demon_hunter.cpp6265–6268: Cull inherits Reap's
    // fragment execution and selects its own native damage child.
    let damage = match spell.id {
        REAP => REAP_DAMAGE,
        CULL => CULL_DAMAGE,
        _ => return,
    };
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
    trigger_soulburst(world, caster, consumed);
    let pending = PendingReap {
        at: spell_cast::now(world) + 0.220,
        target,
        damage,
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

fn trigger_soulburst(world: &mut World, caster: Entity, consumed: usize) {
    if !combat::auras::has_aura(world, caster, 1296615) {
        return;
    }
    let threshold = script_effect_value(world, caster, 1296615, 0).max(0.0) as usize;
    if consumed < threshold {
        return;
    }
    // SimC76f8118 cpp1256–1260,3452–3465,10719–10727: capped bad-luck protection.
    let failures = world.get::<SoulburstFailures>(caster).map_or(0, |state| state.0);
    let chance = (0.0663 + 0.0519 * f64::from(failures)).min(0.3949);
    let proc = combat::random::thread_rng().gen_range(0.0..1.0) < chance;
    world.entity_mut(caster).insert(SoulburstFailures(if proc { 0 } else { failures + 1 }));
    if proc && !spell_cast::try_cast_triggered(world, caster, 1297433, Some(caster), 1) {
        error!(?caster, "native Soulburst aura was refused");
    }
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
    let Some(mut damage) = spell_cast::load_triggered_spell(world, pending.damage, 1) else {
        error!(
            ?caster,
            spell = pending.damage,
            "native Reap/Cull damage missing"
        );
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
        warn!(?caster, target = ?pending.target, spell = pending.damage, "native Reap/Cull damage was refused");
    }
}
