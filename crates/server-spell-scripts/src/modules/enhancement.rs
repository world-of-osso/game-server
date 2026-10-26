//! Native Enhancement execution, owned state and linked payloads.
use super::combat;
use combat::auras::scripts::AuraScripts;
use combat::spell_cast::script_registry::{ContextHitHook, SpellScriptHook};
use combat::spell_cast::scripts::{
    AfterCastHook, reduce_cooldown, script_effect_value, snapshot_proc_modifiers,
};

#[path = "enhancement/enhancement_doom.rs"]
mod enhancement_doom;
#[path = "enhancement/enhancement_elemental_weapons.rs"]
mod enhancement_elemental_weapons;
#[path = "enhancement/enhancement_flurry.rs"]
mod enhancement_flurry;
#[path = "enhancement/enhancement_stormblast.rs"]
mod enhancement_stormblast;
#[path = "enhancement/enhancement_stormwell.rs"]
mod enhancement_stormwell;
#[path = "enhancement/enhancement_imbues.rs"]
mod enhancement_imbues;
#[path = "enhancement/enhancement_modes.rs"]
mod enhancement_modes;
#[path = "enhancement/enhancement_nova.rs"]
mod enhancement_nova;
#[path = "enhancement/enhancement_rod.rs"]
mod enhancement_rod;
#[path = "enhancement/enhancement_spending.rs"]
mod enhancement_spending;
#[path = "enhancement/enhancement_strikes.rs"]
mod enhancement_strikes;
#[path = "enhancement/enhancement_tier.rs"]
mod enhancement_tier;
#[path = "enhancement/enhancement_unleashed.rs"]
mod enhancement_unleashed;
#[path = "enhancement/maelstrom_weapon.rs"]
mod maelstrom_weapon;

pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::WeaponAuraRegistration(register_auras),
    SpellScriptHook::BeforeScriptOnCast(enhancement_stormblast::snapshot),
    SpellScriptHook::AfterWeaponDamage(enhancement_stormblast::after_weapon_damage),
    SpellScriptHook::NativeWeaponMultiplier(enhancement_flurry::weapon_multiplier),
    SpellScriptHook::AfterNativeLaunch(enhancement_flurry::after_launch),
    SpellScriptHook::AfterNativeCastUpdate(enhancement_flurry::update),
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::ContextHit(context_hit),
    SpellScriptHook::CheckNativeSpell(enhancement_strikes::check_cast),
    SpellScriptHook::AfterResolvedHits(enhancement_rod::after_resolution),
    SpellScriptHook::SnapshotNativeAuraAmount(enhancement_rod::snapshot_rod_amount),
    SpellScriptHook::AfterCastUpdate(enhancement_unleashed::update),
    SpellScriptHook::ConsumeCastModifiers(maelstrom_weapon::consume),
    SpellScriptHook::SnapshotsModifier(maelstrom_weapon::snapshots_modifier),
    SpellScriptHook::CastTimeProcs(&[maelstrom_weapon::MODIFIER]),
    SpellScriptHook::AfterWeaponLaunch(enhancement_tier::after_launch),
    SpellScriptHook::AfterWeaponLaunch(enhancement_strikes::cast_ride_the_lightning),
];

fn register_auras(scripts: &mut AuraScripts) {
    maelstrom_weapon::register(scripts);
    enhancement_elemental_weapons::register(scripts);
    enhancement_imbues::register(scripts);
    enhancement_modes::register(scripts);
    enhancement_stormblast::register(scripts);
    enhancement_strikes::register(scripts);
    enhancement_tier::register(scripts);
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        33757 | 318038 => Some(enhancement_imbues::apply),
        470057 => Some(maelstrom_weapon::grant_voltaic_blaze),
        187874 => Some(enhancement_strikes::after_crash),
        188196 | 188443 | 452201 => Some(enhancement_strikes::after_lightning),
        17364 | 115356 | 60103 => Some(enhancement_strikes::grant_elemental_assault),
        _ => None,
    }
}

fn context_hit(spell: u32) -> Option<ContextHitHook> {
    match spell {
        17364 | 115356 | 60103 | 187874 => Some(enhancement_strikes::on_strike),
        _ => None,
    }
}
