//! Explicit missing-script relationships; native records retain payload ownership.
use super::combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Policies(POLICIES)];

const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_mage.cpp4243–4245/4275–4279: native Spike payload
    // and successful-cast consumption of its Icicles/enabler.
    Policy {
        name: "SIMC_FROST_GLACIAL_SPIKE_DAMAGE",
        spec: 64,
        talent: 1246832,
        relationship: Relationship::TriggerPrimaryAfterCast {
            parent: 199786,
            payload: 228600,
        },
    },
    Policy {
        name: "SIMC_FROST_GLACIAL_SPIKE_ICICLE_CONSUMER",
        spec: 64,
        talent: 1246832,
        relationship: Relationship::ConsumeSelfAuraAfterCast {
            parent: 199786,
            payload: 205473,
        },
    },
    Policy {
        name: "SIMC_FROST_GLACIAL_SPIKE_ENABLER_CONSUMER",
        spec: 64,
        talent: 1246832,
        relationship: Relationship::ConsumeSelfAuraAfterCast {
            parent: 199786,
            payload: 1222865,
        },
    },
];
