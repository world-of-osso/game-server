//! Explicit missing-script relationships; native records retain payload ownership.
use super::combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Policies(POLICIES)];

const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_monk.cpp3970–3998: selected Zenith Stomp fires on Zenith.
    Policy {
        name: "SIMC_WW_ZENITH_STOMP_CHILD",
        spec: 269,
        talent: 1272694,
        relationship: Relationship::HitPayload {
            parent: 1249625,
            payload: 1272696,
            self_target: false,
            primary_only: false,
        },
    },
];
