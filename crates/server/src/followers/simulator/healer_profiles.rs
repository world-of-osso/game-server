//! Approved benchmark configurations, never authored roster or canonical DB rows.
use super::*;

pub(crate) const ALTERNATE_SPEC_BRIDGE: &str = "HEALER_BENCHMARK_ALTERNATE_SPEC_BRIDGE";

/// Requested healer spec → authentic same-class actor and its authored source spec.
pub(crate) fn alternate_source(spec: u32) -> Option<(u32, u32)> {
    match spec {
        65 => Some((209057, 66)),
        256 => Some((215011, 257)),
        264 => Some((209053, 262)),
        270 => Some((227734, 269)),
        1468 => Some((215240, 1467)),
        _ => None,
    }
}

pub(crate) fn configure(world: &World, spec: u32) -> Result<Profile, String> {
    let catalog = world.resource::<FollowerCatalog>();
    let Some((entry, source_spec)) = alternate_source(spec) else {
        return catalog
            .profiles
            .iter()
            .find(|profile| {
                profile.spec_id == spec && profile.role == shared::group::GroupRole::Healer
            })
            .cloned()
            .ok_or_else(|| format!("NATIVE_HEALER_PROFILE_ABSENT:{spec}"));
    };
    let mut profile = catalog
        .profiles
        .iter()
        .find(|profile| profile.entry == entry && profile.spec_id == source_spec)
        .cloned()
        .ok_or_else(|| format!("NATIVE_HEALER_SOURCE_PROFILE_ABSENT:{entry}"))?;
    let native_spec = world
        .resource::<crate::spell_info::ClassSpells>()
        .classes
        .iter()
        .find(|class| class.class_id == profile.class_id)
        .is_some_and(|class| class.specs.iter().any(|native| native.spec_id == spec));
    if !native_spec {
        return Err(format!("NATIVE_HEALER_CLASS_SPEC_MISMATCH:{entry}:{spec}"));
    }
    profile.spec_id = spec;
    profile.role = shared::group::GroupRole::Healer;
    // No source DPS/tank decisions become an alternate healer's implicit policy.
    profile.attack_spell = 0;
    profile.cooldown_spell = 0;
    profile.heal_spell = 0;
    profile.taunt_spell = 0;
    profile.form_spell = 0;
    profile.melee = false;
    Ok(profile)
}

pub(super) fn require_policy(world: &World, profile: &Profile, level: u8) -> Result<(), String> {
    if alternate_source(profile.spec_id).is_none() {
        return Ok(());
    }
    if !world
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .any(|entry| entry.available(profile.spec_id, level))
    {
        return Err("NATIVE_HEALER_POLICY_ABSENT".into());
    }
    // The policy owner must supply this normalization input; zero is not an idle metric.
    if profile.heal_spell == 0 {
        return Err("NATIVE_HEALER_PRIMARY_HEAL_ABSENT".into());
    }
    Ok(())
}
