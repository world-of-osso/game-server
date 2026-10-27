//! Icy Death Torrent follows actual critical weapon swings.
use bevy::prelude::*;
use shared::components::{ActiveSpec, WeaponType};

use super::combat;
use combat::auras;
use combat::combat::AutoAttack;
use combat::random;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::try_cast_triggered;
use combat::spell_info::SpellStore;
use combat::stats::CharacterStats;

const TALENT: u32 = 435010;
const DAMAGE: u32 = 439539;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterWhiteHit(roll_torrent)];

fn roll_torrent(world: &mut World, caster: Entity, critical: bool) {
    if !critical
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 251)
        || !auras::has_aura(world, caster, TALENT)
    {
        return;
    }
    let Some(target) = world.get::<AutoAttack>(caster).map(|attack| attack.victim) else {
        return;
    };
    let two_hand = world
        .get::<CharacterStats>(caster)
        .is_some_and(|stats| stats.main_hand.kind == WeaponType::TwoHand);
    let chance = world
        .resource::<SpellStore>()
        .get(TALENT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Icy Death Torrent proc metadata")
        .proc_chance as f32
        / 100.0;
    // SimC76f8118 sc_death_knight.cpp7657–7665/7715–7724: critical white
    // hits roll the native chance, divided by effect1 × 0.1 for dual wield.
    let divisor = if two_hand {
        1.0
    } else {
        script_effect_value(world, caster, TALENT, 0) * 0.1
    };
    assert!(
        divisor > 0.0,
        "native Icy Death Torrent divisor must be positive"
    );
    if random::random::<f32>() < chance / divisor {
        assert!(
            try_cast_triggered(world, caster, DAMAGE, Some(target), 1),
            "native Icy Death Torrent refused"
        );
    }
}
