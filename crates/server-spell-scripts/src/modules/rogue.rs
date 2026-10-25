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
    // SimC76f8118 sc_rogue.cpp3771/3836/4810/6034/6890 call8284–8305.
    Policy {
        name: "SIMC_OUTLAW_RESTLESS_BLADES",
        spec: 260,
        talent: 79096,
        relationship: Relationship::SpenderCooldownRefund {
            resource: PowerType::ComboPoints,
            parents: &[2098, 315341, 51690, 315496, 441776],
            cooldowns: &[
                13750, 315341, 13877, 271877, 195457, 381989, 51690, 1214909, 2983,
            ],
            tenths_effect: 0,
            bonus_states: &[1214935, 1214937],
            bonus_spell: 1214937,
            bonus_effect: 3,
        },
    },
];
