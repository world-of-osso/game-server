//! Dire Summons shortens only the owner's already-running native Howl clock.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterElementalLaunch(reduce_howl)];

const DIRE_SUMMONS: u32 = 472352;
const HOWL_COOLDOWN: u32 = 471877;
// SimC76f8118 sc_hunter.cpp4966/6127/6624: Cobra, melee Focus
// spenders (including Eagle/Swipe), and specialization-qualified Kill Command.
const RECOVERY_EFFECTS: &[(u32, u8)] = &[
    (34026, 0),
    (259489, 1),
    (193455, 2),
    (186270, 3),
    (265189, 3),
    (1262293, 3),
];

fn reduce_howl(world: &mut World, caster: Entity, spell: &SpellInfo, _foreground: bool) {
    // Cobra's shared execute also runs for native Snakeskin background Cobras.
    // sc_hunter.cpp4966/5035–5047 does not restrict recovery to foreground casts.
    if !has_talent(world, caster, DIRE_SUMMONS) {
        return;
    }
    let Some(&(_, effect)) = RECOVERY_EFFECTS.iter().find(|(root, _)| *root == spell.id) else {
        return;
    };
    let expiry = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == HOWL_COOLDOWN && aura.caster == caster)
            .and_then(|aura| aura.expires_at)
    });
    let Some(expiry) = expiry else {
        return;
    };
    let reduction = f64::from(script_effect_value(world, caster, DIRE_SUMMONS, effect)) / 1000.0;
    let remaining = expiry - auras::aura_now(world) - reduction;
    shorten_or_expire_clock(world, caster, remaining);
}

fn shorten_or_expire_clock(world: &mut World, caster: Entity, remaining: f64) {
    // Position2531 follows Howl consumption2530, matching hunter6604→6624.
    // buff_t::extend_duration expires an exhausted clock through its normal
    // callback; an inactive clock is never started or replaced.
    if remaining <= 0.0 {
        auras::remove_auras(world, caster, RemoveReason::Expired, |aura| {
            aura.spell_id == HOWL_COOLDOWN && aura.caster == caster
        });
    } else {
        auras::set_owned_duration(
            world,
            caster,
            caster,
            HOWL_COOLDOWN,
            (remaining * 1000.0).round() as u32,
        );
    }
}
