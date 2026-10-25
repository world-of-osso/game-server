//! Explicit missing-script relationships; native records retain payload ownership.
use super::combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Policies(POLICIES)];

const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_hunter.cpp6702–6703: selected Nature's Ally spawns
    // through its native Effect188 record; native1282474 supplies count/lifetime.
    Policy {
        name: "SIMC_BM_WRATH_NATURES_ALLY_SUMMON",
        spec: 253,
        talent: 1273043,
        relationship: Relationship::TriggerPrimaryAfterCast {
            parent: 19574,
            payload: 1282474,
        },
    },
    // SimC76f8118 sc_hunter.cpp6746–6750: every active stable pet applies
    // Bloodshed after the summon above. Native321538 owns the periodic payload.
    Policy {
        name: "SIMC_BM_WRATH_BLOODSHED",
        spec: 253,
        talent: 1272099,
        relationship: Relationship::OwnedPetSpellAfterCast {
            parent: 19574,
            payload: 321538,
        },
    },
    // SimC76f8118 sc_hunter.cpp5089/4972,3843–3846: successful Barbed/Cobra
    // arms selected Nature's Ally; native1276720 owns its value and lifetime.
    Policy {
        name: "SIMC_BM_BARBED_NATURES_ALLY",
        spec: 253,
        talent: 1273126,
        relationship: Relationship::ApplySelfAuraAfterCast {
            parent: 217200,
            payload: 1276720,
        },
    },
    Policy {
        name: "SIMC_BM_COBRA_NATURES_ALLY",
        spec: 253,
        talent: 1273126,
        relationship: Relationship::ApplySelfAuraAfterCast {
            parent: 193455,
            payload: 1276720,
        },
    },
    // SimC76f8118 sc_hunter.cpp6633: consume only after Kill Command executes.
    Policy {
        name: "SIMC_BM_KILL_COMMAND_NATURES_ALLY",
        spec: 253,
        talent: 1273126,
        relationship: Relationship::ConsumeSelfAuraAfterCast {
            parent: 34026,
            payload: 1276720,
        },
    },
];
