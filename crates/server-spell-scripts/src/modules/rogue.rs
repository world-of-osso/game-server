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

const RESTLESS_COOLDOWNS: &[u32] = &[
    13750, 315341, 13877, 271877, 195457, 381989, 51690, 1214909, 2983,
];

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
            parents: &[2098, 315341, 51690, 315496],
            cooldowns: RESTLESS_COOLDOWNS,
            tenths_effect: 0,
            bonus_points_effect: None,
        },
    },
    // INFERRED_WCL: T9M3/fight10 actor277 Spree18.836→43.563s requires
    // more CDR than unbonused Coup points allow. DB2 441423 effect3 and
    // SimC76f8118 sc_rogue.cpp6863–6890 include Coup's five effective points.
    Policy {
        name: "SIMC_OUTLAW_COUP_RESTLESS_BLADES",
        spec: 260,
        talent: 79096,
        relationship: Relationship::SpenderCooldownRefund {
            resource: PowerType::ComboPoints,
            parents: &[441776],
            cooldowns: RESTLESS_COOLDOWNS,
            tenths_effect: 0,
            bonus_points_effect: Some((441423, 2)),
        },
    },
];
