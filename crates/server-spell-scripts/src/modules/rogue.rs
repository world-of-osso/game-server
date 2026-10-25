//! Explicit missing-script relationships; native records retain payload ownership.
use super::combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};
use shared::components::PowerType;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Policies(POLICIES)];

// sc_rogue.cpp3787/4386/4439/6891 call the shared8704–8716 extension.
const CUT_TO_THE_CHASE: Relationship = Relationship::SpenderAuraExtension {
    resource: PowerType::ComboPoints,
    parents: &[2098, 32645, 196819, 441776],
    payload: 315496,
    seconds_effect: 0,
};

const POLICIES: &[Policy] = &[
    Policy {
        name: "SIMC_ROGUE_CUT_TO_THE_CHASE",
        spec: 259,
        talent: 51667,
        relationship: CUT_TO_THE_CHASE,
    },
    Policy {
        name: "SIMC_ROGUE_CUT_TO_THE_CHASE",
        spec: 260,
        talent: 51667,
        relationship: CUT_TO_THE_CHASE,
    },
    Policy {
        name: "SIMC_ROGUE_CUT_TO_THE_CHASE",
        spec: 261,
        talent: 51667,
        relationship: CUT_TO_THE_CHASE,
    },
];
