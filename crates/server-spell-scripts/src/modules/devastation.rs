//! Explicit missing-script relationships; native records retain payload ownership.
use super::combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Policies(POLICIES)];

const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_evoker.cpp5592–5599: starting Eternity Surge grants
    // selected Azure Sweep. Native1265871 supplies its action override and damage mod.
    Policy {
        name: "SIMC_DEVASTATION_SURGE_AZURE_SWEEP",
        spec: 1467,
        talent: 1265867,
        relationship: Relationship::ApplySelfAuraAfterCast {
            parent: 359073,
            payload: 1265871,
        },
    },
    Policy {
        name: "SIMC_DEVASTATION_SURGE_VARIANT_AZURE_SWEEP",
        spec: 1467,
        talent: 1265867,
        relationship: Relationship::ApplySelfAuraAfterCast {
            parent: 382411,
            payload: 1265871,
        },
    },
    // SimC76f8118 sc_evoker.cpp6584–6595: one successful Disintegrate
    // spends one earned Mass Disintegrate use, not one per secondary recipient.
    Policy {
        name: "SIMC_DEVASTATION_MASS_DISINTEGRATE_CONSUMER",
        spec: 1467,
        talent: 436335,
        relationship: Relationship::SpendSelfAuraStackAfterCast {
            parent: 356995,
            payload: 436336,
        },
    },
    // SimC76f8118 sc_evoker.cpp6241–6245: Deep Breath's main impact
    // executes native353759; DB2 supplies its periodic damage and recipients.
    Policy {
        name: "SIMC_DEVASTATION_DEEP_BREATH_PAYLOAD",
        spec: 1467,
        talent: 357210,
        relationship: Relationship::TriggerPrimaryAfterCast {
            parent: 357210,
            payload: 353759,
        },
    },
    Policy {
        name: "SIMC_DEVASTATION_MANEUVERABLE_DEEP_BREATH_PAYLOAD",
        spec: 1467,
        talent: 433871,
        relationship: Relationship::TriggerPrimaryAfterCast {
            parent: 433874,
            payload: 353759,
        },
    },
];
